//! Entering and exiting states: building the chart, applying a batch of transition requests,
//! and keeping the live set consistent with the tree when a hook panics.

use super::{Ancestors, ErasedState, Frame, Nil, Node, Registry, Request, Runtime};
use crate::context::Hook;

/// Upper bound on transitions applied in one batch; catches `on_enter` hooks that keep
/// requesting transitions in a cycle.
const MAX_TRANSITIONS_PER_BATCH: usize = 10_000;

/// While states are entered: the rest of the path to a transition target below the state being
/// entered (empty when only initial children are entered), and the target's instance, taken
/// when its turn comes.
struct Towards<'t> {
    path: &'t [usize],
    target: &'t mut Option<Box<dyn ErasedState>>,
}

impl<'t> Towards<'t> {
    /// Only initial children.
    fn nowhere(target: &'t mut Option<Box<dyn ErasedState>>) -> Self {
        Towards { path: &[], target }
    }
}

/// Re-synchronises the live set with the tree if a step of an operation is interrupted by a
/// panic in a hook.
///
/// Every state in the tree has been entered (at least partly) and not exited, because a state
/// is placed in the tree when it is entered and detached before it is exited. The one exception
/// is a new root whose predecessor's `on_exit` panicked during a restart: it is in the tree but
/// not entered yet, and [`Runtime::root_pending_enter`] says so. A state whose exit was
/// interrupted, or a detached descendant that was never exited, may still be in the live set.
/// Rebuilding the live set from the tree runs no user code, so it is safe during unwinding.
///
/// It also drops the requests recorded but not applied yet (by a traversal, or by `on_enter`
/// hooks during a batch; the queued ones go with the batch's own queue). They would never be
/// applied anyway, but their targets may hold resources, which are released while the panic
/// unwinds instead of at the next operation.
pub(super) struct ResyncOnUnwind<'a> {
    pub(super) reg: &'a Registry,
    pub(super) rt: &'a mut Runtime,
    pub(super) root: &'a mut Node,
    pub(super) armed: bool,
}

impl Drop for ResyncOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.rt.live.clear();
            self.reg.mark_live(self.rt, self.root);
            self.rt.requests.clear();
        }
    }
}

impl Registry {
    /// Adds `node` and its descendants to the live set.
    fn mark_live(&self, rt: &mut Runtime, node: &Node) {
        if node.is_vacant() {
            return;
        }
        rt.live.insert(self.states[node.idx].ty, node.id);
        for child in &node.regions {
            self.mark_live(rt, child);
        }
    }

    /// Enters the root (index 0) with its initial descendants.
    pub(super) fn enter_root(&self, rt: &mut Runtime, root: Box<dyn ErasedState>) -> Node {
        let mut node = self.place(rt, 0, root);
        self.enter_placed(rt, &mut Nil, &mut node, Towards::nowhere(&mut None));
        node
    }

    /// Enters the root, which is already in the tree. From here on it counts as entered, even
    /// if its `on_enter` panics.
    pub(super) fn enter_placed_root(&self, rt: &mut Runtime, root: &mut Node) {
        rt.root_pending_enter = false;
        self.enter_placed(rt, &mut Nil, root, Towards::nowhere(&mut None));
    }

    /// A new node for `state` (of registry index `idx`), live from now on, with every region
    /// vacant.
    fn place(&self, rt: &mut Runtime, idx: usize, state: Box<dyn ErasedState>) -> Node {
        let regions = self.states[idx]
            .regions
            .iter()
            .map(|_| Node::vacant())
            .collect();
        self.place_with(rt, idx, state, regions)
    }

    /// [`place`](Self::place) with the given vacant regions (one per region of `idx`).
    fn place_with(
        &self,
        rt: &mut Runtime,
        idx: usize,
        state: Box<dyn ErasedState>,
        regions: Vec<Node>,
    ) -> Node {
        debug_assert!(
            regions.len() == self.states[idx].regions.len() && regions.iter().all(Node::is_vacant)
        );
        Node {
            state,
            idx,
            id: rt.activate(self.states[idx].ty),
            regions,
        }
    }

    /// Enters `idx` into the vacant `slot`: builds it (unless `instance` is given) from its
    /// parent, places it in the tree and enters it ([`enter_placed`](Self::enter_placed)).
    fn enter(
        &self,
        rt: &mut Runtime,
        anc: &mut dyn Ancestors,
        slot: &mut Node,
        idx: usize,
        instance: Option<Box<dyn ErasedState>>,
        towards: Towards<'_>,
    ) {
        let state = match instance {
            Some(state) => state,
            None => {
                // Checked when the chart was built: every state the library may have to build
                // (initial children, composites) has a constructor.
                let ctor = self.states[idx]
                    .ctor
                    .as_ref()
                    .expect("checked at construction");
                ctor(anc.parent().expect("only the root has no parent"))
            }
        };
        *slot = self.place(rt, idx, state);
        self.enter_placed(rt, anc, slot, towards);
    }

    /// Runs `on_enter` of a node that is already in the tree, then enters one child per region
    /// in declaration order. Each state is in the tree before its `on_enter` runs, so a panic
    /// in a hook never leaves an entered state outside the tree.
    fn enter_placed(
        &self,
        rt: &mut Runtime,
        anc: &mut dyn Ancestors,
        node: &mut Node,
        towards: Towards<'_>,
    ) {
        let Node {
            state,
            regions,
            idx,
            id,
        } = node;
        state.erased_enter(Hook {
            anc: &mut *anc,
            reg: self,
            rt: &mut *rt,
            id: *id,
        });

        let info = &self.states[*idx];
        let mut frame = Frame {
            head: &mut **state,
            tail: anc,
        };
        for (region, slot) in regions.iter_mut().enumerate() {
            match towards.path.split_first() {
                Some((&next, rest)) if self.states[next].region_in_parent == region => {
                    let instance = if rest.is_empty() {
                        towards.target.take()
                    } else {
                        None
                    };
                    let towards = Towards {
                        path: rest,
                        target: &mut *towards.target,
                    };
                    self.enter(rt, &mut frame, slot, next, instance, towards);
                }
                _ => {
                    let initial = info.regions[region];
                    let towards = Towards::nowhere(&mut *towards.target);
                    self.enter(rt, &mut frame, slot, initial, None, towards);
                }
            }
        }
    }

    /// Applies every recorded request in order, plus any requested by `on_enter` hooks while
    /// applying them (appended to the end of the queue).
    ///
    /// If a hook panics, the panic propagates and the rest of the batch is dropped; the caller
    /// keeps the tree consistent with the live set (see [`ResyncOnUnwind`]).
    pub(super) fn apply_all(&self, rt: &mut Runtime, root: &mut Node) {
        // A panic drops the taken queue with the rest of the batch; the next batch then starts
        // with a fresh buffer.
        let mut queue = std::mem::take(&mut rt.queue);
        queue.extend(rt.requests.drain(..));
        let mut applied = 0usize;
        while let Some(request) = queue.pop_front() {
            // Spec item 10: drop requests made by a state exited earlier in this batch.
            if !rt.is_live_instance(request.requester_ty, request.requester) {
                continue;
            }
            applied += 1;
            assert!(
                applied <= MAX_TRANSITIONS_PER_BATCH,
                "more than {MAX_TRANSITIONS_PER_BATCH} transitions in one batch: do `on_enter` \
                 hooks keep requesting transitions in a cycle? (the last request: `{}` \
                 requested `{}`)",
                self.name_of(request.requester_ty),
                self.states[request.target_idx].name,
            );
            self.apply(rt, root, request);
            queue.extend(rt.requests.drain(..));
        }
        rt.queue = queue;
    }

    fn apply(&self, rt: &mut Runtime, root: &mut Node, request: Request) {
        let path = &self.states[request.target_idx].path;

        // k = first index on the target's path whose state is not active. If the whole path is
        // active, the target itself is re-entered (external self-transition); k == 0 only when
        // the target is the root type.
        let k = self.active_depth(root, path).min(path.len() - 1);

        if k == 0 {
            self.restart(rt, root, request.target);
        } else {
            let mut target = Some(request.target);
            self.replace_in(rt, root, &mut Nil, &path[1..k], &path[k..], &mut target);
        }
    }

    /// Exits the whole chart and enters `new_root` with its initial descendants.
    ///
    /// Like every exited subtree, the old root's regions are all detached (the root keeps
    /// vacant ones) before any of them is exited, so a panic while one is exited drops the
    /// later ones without `on_exit` instead of leaving them active under the old root.
    ///
    /// The root slot is never vacant: the new root takes the old one's place once the old
    /// root's descendants are exited, just before the old root's own `on_exit`. Until its own
    /// `on_enter` starts it is marked as not entered yet ([`Runtime::root_pending_enter`]), so
    /// if the old root's `on_exit` panics, the next operation enters it first.
    fn restart(&self, rt: &mut Runtime, root: &mut Node, new_root: Box<dyn ErasedState>) {
        let vacant = root.regions.iter().map(|_| Node::vacant()).collect();
        let detached = std::mem::replace(&mut root.regions, vacant);
        {
            let mut frame = Frame {
                head: &mut *root.state,
                tail: &mut Nil,
            };
            for node in detached {
                self.exit_detached(rt, node, &mut frame);
            }
        }
        // The new root has the same regions, so it takes over the vacant ones.
        let regions = std::mem::take(&mut root.regions);
        let old = std::mem::replace(root, self.place_with(rt, 0, new_root, regions));
        rt.root_pending_enter = true;
        self.exit_detached(rt, old, &mut Nil);
        self.enter_placed_root(rt, root);
    }

    /// Walks down `descend` (active states below `node`), then replaces the active subtree of
    /// the region that contains `entering[0]` with a fresh one leading to the target.
    fn replace_in(
        &self,
        rt: &mut Runtime,
        node: &mut Node,
        anc: &mut dyn Ancestors,
        descend: &[usize],
        entering: &[usize],
        target: &mut Option<Box<dyn ErasedState>>,
    ) {
        let Node { state, regions, .. } = node;
        let mut frame = Frame {
            head: &mut **state,
            tail: anc,
        };
        match descend.split_first() {
            Some((&next, rest)) => {
                let region = self.states[next].region_in_parent;
                self.replace_in(rt, &mut regions[region], &mut frame, rest, entering, target);
            }
            None => {
                let slot = &mut regions[self.states[entering[0]].region_in_parent];
                self.exit_slot(rt, slot, &mut frame);
                let instance = if entering.len() == 1 {
                    target.take()
                } else {
                    None
                };
                let towards = Towards {
                    path: &entering[1..],
                    target,
                };
                self.enter(rt, &mut frame, slot, entering[0], instance, towards);
            }
        }
    }

    /// Detaches the subtree in `slot` from the tree (leaving the region vacant), then exits it.
    fn exit_slot(&self, rt: &mut Runtime, slot: &mut Node, anc: &mut dyn Ancestors) {
        let node = std::mem::replace(slot, Node::vacant());
        self.exit_detached(rt, node, anc);
    }

    /// Post-order `on_exit` of a subtree that is no longer in the tree: descendants first
    /// (regions in declaration order), then `node`. Each state is dropped right after its
    /// `on_exit`, so everything exited is gone before anything new is entered.
    fn exit_detached(&self, rt: &mut Runtime, mut node: Node, anc: &mut dyn Ancestors) {
        if node.is_vacant() {
            return;
        }
        let Node {
            state,
            regions,
            idx,
            id,
        } = &mut node;
        {
            let mut frame = Frame {
                head: &mut **state,
                tail: &mut *anc,
            };
            for slot in regions.iter_mut() {
                self.exit_slot(rt, slot, &mut frame);
            }
        }
        state.erased_exit(Hook {
            anc,
            reg: self,
            rt: &mut *rt,
            id: *id,
        });
        rt.deactivate(self.states[*idx].ty, *id);
    }
}
