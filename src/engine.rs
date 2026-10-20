//! Type-erased internals: the static registry built from the `define` functions, the runtime
//! bookkeeping, the active-configuration tree, the ancestor frames, and the traversals.

use crate::context::{Context, ExitContext, Hook};
use crate::definition::Definition;
use crate::state::State;
use crate::trigger::{Event, Trigger};
use std::any::{Any, TypeId, type_name};
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};

// ---------------------------------------------------------------------------------------------
// Erased states
// ---------------------------------------------------------------------------------------------

/// Object-safe view of a [`State`]. Blanket-implemented, never implemented by users.
pub(crate) trait ErasedState: Any + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn erased_enter(&mut self, hook: Hook<'_>);
    fn erased_exit(&mut self, hook: Hook<'_>);
    fn erased_update(&mut self, hook: Hook<'_>);
}

impl<S: State> ErasedState for S {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn erased_enter(&mut self, hook: Hook<'_>) {
        self.on_enter(&mut Context::new(hook));
    }
    fn erased_exit(&mut self, hook: Hook<'_>) {
        self.on_exit(&mut ExitContext::new(hook));
    }
    fn erased_update(&mut self, hook: Hook<'_>) {
        self.update(&mut Context::new(hook));
    }
}

// ---------------------------------------------------------------------------------------------
// Ancestor frames
// ---------------------------------------------------------------------------------------------

/// The ancestors of the state whose hook is running, nearest first.
///
/// Implemented by a linked list of stack frames ([`Frame`] ... [`Nil`]). Each traversal step
/// splits a node into its state and its regions (disjoint borrows), pushes `&mut state` as a new
/// frame on the Rust call stack, and recurses into the regions. No `unsafe`, no `RefCell`.
pub(crate) trait Ancestors {
    fn parent(&self) -> Option<&dyn ErasedState>;
    fn parent_mut(&mut self) -> Option<&mut dyn ErasedState>;
    fn find(&self, ty: TypeId) -> Option<&dyn ErasedState>;
    fn find_mut(&mut self, ty: TypeId) -> Option<&mut dyn ErasedState>;
}

/// No ancestors (the root's view).
pub(crate) struct Nil;

impl Ancestors for Nil {
    fn parent(&self) -> Option<&dyn ErasedState> {
        None
    }
    fn parent_mut(&mut self) -> Option<&mut dyn ErasedState> {
        None
    }
    fn find(&self, _: TypeId) -> Option<&dyn ErasedState> {
        None
    }
    fn find_mut(&mut self, _: TypeId) -> Option<&mut dyn ErasedState> {
        None
    }
}

/// One ancestor (`head`) followed by its own ancestors (`tail`).
struct Frame<'a> {
    head: &'a mut dyn ErasedState,
    tail: &'a mut dyn Ancestors,
}

impl Ancestors for Frame<'_> {
    fn parent(&self) -> Option<&dyn ErasedState> {
        Some(&*self.head)
    }
    fn parent_mut(&mut self) -> Option<&mut dyn ErasedState> {
        Some(&mut *self.head)
    }
    fn find(&self, ty: TypeId) -> Option<&dyn ErasedState> {
        if self.head.as_any().type_id() == ty {
            Some(&*self.head)
        } else {
            self.tail.find(ty)
        }
    }
    fn find_mut(&mut self, ty: TypeId) -> Option<&mut dyn ErasedState> {
        if self.head.as_any().type_id() == ty {
            Some(&mut *self.head)
        } else {
            self.tail.find_mut(ty)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Runtime bookkeeping
// ---------------------------------------------------------------------------------------------

/// A transition request, recorded during a traversal and applied after it.
pub(crate) struct Request {
    /// Type and instance id of the requesting state. The id, not the type, decides whether the
    /// requester is still active: a re-entered state is a new instance.
    pub(crate) requester_ty: TypeId,
    pub(crate) requester: u64,
    /// Registry index of the target type (validated when the request is made).
    pub(crate) target_idx: usize,
    pub(crate) target: Box<dyn ErasedState>,
}

/// Mutable state shared by every traversal of one chart.
#[derive(Default)]
pub(crate) struct Runtime {
    /// Requests of the current traversal or transition, in request order.
    pub(crate) requests: Vec<Request>,
    /// Active state types and their instance ids. A state is live from just before its
    /// `on_enter` until just after its `on_exit`.
    live: HashMap<TypeId, u64>,
    next_id: u64,
}

impl Runtime {
    pub(crate) fn is_live(&self, ty: TypeId) -> bool {
        self.live.contains_key(&ty)
    }

    fn is_live_instance(&self, ty: TypeId, id: u64) -> bool {
        self.live.get(&ty) == Some(&id)
    }

    fn activate(&mut self, ty: TypeId) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.live.insert(ty, id);
        id
    }

    fn deactivate(&mut self, ty: TypeId, id: u64) {
        if self.is_live_instance(ty, id) {
            self.live.remove(&ty);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Registry (static hierarchy knowledge)
// ---------------------------------------------------------------------------------------------

/// Builds a child from its (already entered) parent.
pub(crate) type Ctor = Box<dyn Fn(&dyn ErasedState) -> Box<dyn ErasedState>>;

/// Runs one state's handler for one trigger type and appends the output to a `Vec<E::Output>`.
pub(crate) type HandlerFn = fn(&mut dyn ErasedState, &dyn Any, &mut dyn Any, Hook<'_>);

/// Registers a child type (monomorphised `register::<C>`); returns its registry index.
pub(crate) type RegisterFn = fn(&mut Registry, Option<usize>, usize, Option<Ctor>) -> usize;

/// Upper bound on transitions applied in one batch; catches `on_enter` hooks that keep
/// requesting transitions in a cycle.
pub(crate) const MAX_TRANSITIONS_PER_BATCH: usize = 10_000;

struct StateInfo {
    ty: TypeId,
    /// Index of this state's region in its parent.
    region_in_parent: usize,
    /// Registry index of each region's initial child.
    regions: Vec<usize>,
    triggers: Vec<(TypeId, HandlerFn)>,
    /// `None` only for the root and for leaves registered with `child_explicit`.
    ctor: Option<Ctor>,
    /// Registry indices from the root down to this state (inclusive).
    path: Box<[usize]>,
}

/// Every state type of one chart, registered once by [`Registry::build`].
#[derive(Default)]
pub(crate) struct Registry {
    states: Vec<StateInfo>,
    index: HashMap<TypeId, usize>,
}

/// Registers `S` (and, recursively, everything its `define` declares); returns its index.
pub(crate) fn register<S: State>(
    reg: &mut Registry,
    parent: Option<usize>,
    region_in_parent: usize,
    ctor: Option<Ctor>,
) -> usize {
    let ty = TypeId::of::<S>();
    // Spec item 11, always on (user decision D1). `child::<C>()` only compiles when
    // `C::Parent == Self`, so the only way to get here twice is a parent listing the same child
    // twice (in one region or in two). The check costs one hash lookup per state, once per
    // `Statechart::new`, and a duplicate would make transition targets ambiguous.
    assert!(
        !reg.index.contains_key(&ty),
        "State type `{}` appears more than once in the statechart hierarchy",
        type_name::<S>()
    );

    let mut def = Definition::<S>::new();
    S::define(&mut def);
    let (regions, triggers) = def.into_parts();

    assert!(
        parent.is_none() || ctor.is_some() || regions.is_empty(),
        "State `{0}` has regions but was registered with `child_explicit`: a composite state can \
         be entered on the way to one of its descendants, so it needs a constructor; use \
         `child::<{0}>()` (requires `Default`) or `child_with(...)`",
        type_name::<S>()
    );

    let idx = reg.states.len();
    let path = match parent {
        Some(p) => reg.states[p].path.iter().copied().chain([idx]).collect(),
        None => Box::from([idx]),
    };
    reg.states.push(StateInfo {
        ty,
        region_in_parent,
        regions: Vec::with_capacity(regions.len()),
        triggers,
        ctor,
        path,
    });
    reg.index.insert(ty, idx);

    for (region, def) in regions.into_iter().enumerate() {
        let mut initial = None;
        for child in def.children {
            let child_idx = (child.register)(reg, Some(idx), region, child.ctor);
            initial.get_or_insert(child_idx);
        }
        let initial = initial.expect("a region is declared by naming its initial child");
        reg.states[idx].regions.push(initial);
    }
    idx
}

/// Type-erased trigger handler for `S: Trigger<E>`.
pub(crate) fn call_handler<S: Trigger<E>, E: Event>(
    state: &mut dyn ErasedState,
    event: &dyn Any,
    outputs: &mut dyn Any,
    hook: Hook<'_>,
) {
    let state = state
        .as_any_mut()
        .downcast_mut::<S>()
        .expect("handler registered for another state type");
    let event = event.downcast_ref::<E>().expect("event type mismatch");
    let outputs = outputs
        .downcast_mut::<Vec<E::Output>>()
        .expect("output type mismatch");
    outputs.push(state.handle(event, &mut Context::new(hook)));
}

/// A trigger being dispatched.
pub(crate) struct Dispatch<'e> {
    pub(crate) event_ty: TypeId,
    pub(crate) event: &'e dyn Any,
    /// A `Vec<E::Output>`.
    pub(crate) outputs: &'e mut dyn Any,
}

// ---------------------------------------------------------------------------------------------
// Active configuration
// ---------------------------------------------------------------------------------------------

/// An active state and the active child of each of its regions (`regions[i]` is region `i`).
pub(crate) struct Node {
    pub(crate) state: Box<dyn ErasedState>,
    /// Registry index of the state's type.
    idx: usize,
    /// Instance id.
    id: u64,
    regions: Vec<Node>,
}

impl Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.state)?;
        match self.regions.as_slice() {
            [] => Ok(()),
            [only] => write!(f, " -> {only:?}"),
            many => {
                f.write_str(" -> [")?;
                for (i, child) in many.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{child:?}")?;
                }
                f.write_str("]")
            }
        }
    }
}

impl Node {
    /// Post-order (deepest first within a branch, regions in declaration order), excluding self.
    pub(crate) fn collect_descendant_types(&self, out: &mut Vec<TypeId>) {
        for child in &self.regions {
            child.collect_descendant_types(out);
            out.push(child.state.as_any().type_id());
        }
    }
}

impl Registry {
    /// Registers the whole hierarchy below the root `R`.
    pub(crate) fn build<R: State>() -> Self {
        let mut reg = Registry::default();
        register::<R>(&mut reg, None, 0, None);
        reg
    }

    pub(crate) fn index_of(&self, ty: TypeId) -> Option<usize> {
        self.index.get(&ty).copied()
    }

    /// Follows the static path of `ty` through the active tree.
    pub(crate) fn find_active<'n>(&self, root: &'n Node, ty: TypeId) -> Option<&'n Node> {
        let path = &self.states[self.index_of(ty)?].path;
        let (first, rest) = path.split_first()?;
        if root.idx != *first {
            return None;
        }
        let mut node = root;
        for &idx in rest {
            let child = &node.regions[self.states[idx].region_in_parent];
            if child.idx != idx {
                return None;
            }
            node = child;
        }
        Some(node)
    }

    fn handler(&self, idx: usize, event: TypeId) -> Option<HandlerFn> {
        self.states[idx]
            .triggers
            .iter()
            .find(|(e, _)| *e == event)
            .map(|(_, h)| *h)
    }

    /// Enters the root `R` (index 0) with its initial descendants.
    pub(crate) fn enter_root(&self, rt: &mut Runtime, root: Box<dyn ErasedState>) -> Node {
        self.enter(rt, &mut Nil, 0, Some(root), &[], &mut None)
    }

    /// Enters `idx`: builds it (unless `instance` is given) from its parent, runs `on_enter`,
    /// then enters one child per region in declaration order. `towards` is the rest of the path
    /// to a transition target below `idx` (empty when only initial children are entered); the
    /// target's instance is taken from `target` when its turn comes.
    fn enter(
        &self,
        rt: &mut Runtime,
        anc: &mut dyn Ancestors,
        idx: usize,
        instance: Option<Box<dyn ErasedState>>,
        towards: &[usize],
        target: &mut Option<Box<dyn ErasedState>>,
    ) -> Node {
        let info = &self.states[idx];
        let mut state = match instance {
            Some(state) => state,
            None => {
                // Checked when the chart was built: every state the library may have to build
                // (initial children, composites) has a constructor.
                let ctor = info.ctor.as_ref().expect("checked at construction");
                ctor(anc.parent().expect("only the root has no parent"))
            }
        };
        let id = rt.activate(info.ty);
        state.erased_enter(Hook {
            anc: &mut *anc,
            reg: self,
            rt: &mut *rt,
            id,
        });

        let mut regions = Vec::with_capacity(info.regions.len());
        let mut frame = Frame {
            head: &mut *state,
            tail: anc,
        };
        for (region, &initial) in info.regions.iter().enumerate() {
            let child = match towards.split_first() {
                Some((&next, rest)) if self.states[next].region_in_parent == region => {
                    let instance = if rest.is_empty() { target.take() } else { None };
                    self.enter(rt, &mut frame, next, instance, rest, target)
                }
                _ => self.enter(rt, &mut frame, initial, None, &[], target),
            };
            regions.push(child);
        }

        Node {
            state,
            idx,
            id,
            regions,
        }
    }

    /// Applies every queued request in order, plus any requested by `on_enter` hooks while
    /// applying them (appended to the end of the queue).
    pub(crate) fn run_batch(&self, rt: &mut Runtime, root: &mut Node) {
        let mut queue: VecDeque<Request> = std::mem::take(&mut rt.requests).into();
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
                 hooks keep requesting transitions in a cycle?"
            );
            self.apply(rt, root, request);
            queue.extend(rt.requests.drain(..));
        }
    }

    fn apply(&self, rt: &mut Runtime, root: &mut Node, request: Request) {
        let path = &self.states[request.target_idx].path;

        // k = first index on the target's path whose state is not active. If the whole path is
        // active, the target itself is re-entered (external self-transition); k == 0 only when
        // the target is the root type.
        let mut k = path.len() - 1;
        let mut node = &*root;
        for (i, &idx) in path.iter().enumerate().skip(1) {
            let child = &node.regions[self.states[idx].region_in_parent];
            if child.idx != idx {
                k = i;
                break;
            }
            node = child;
        }

        let mut target = Some(request.target);
        if k == 0 {
            self.exit_node(rt, root, &mut Nil);
            *root = self.enter(rt, &mut Nil, path[0], target.take(), &[], &mut None);
        } else {
            self.replace_in(rt, root, &mut Nil, &path[1..k], &path[k..], &mut target);
        }
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
                let region = self.states[entering[0]].region_in_parent;
                self.exit_node(rt, &mut regions[region], &mut frame);
                let instance = if entering.len() == 1 {
                    target.take()
                } else {
                    None
                };
                regions[region] = self.enter(
                    rt,
                    &mut frame,
                    entering[0],
                    instance,
                    &entering[1..],
                    target,
                );
            }
        }
    }

    /// Pre-order `update`: parent before children, regions in declaration order.
    pub(crate) fn update_node(&self, rt: &mut Runtime, node: &mut Node, anc: &mut dyn Ancestors) {
        let Node {
            state, regions, id, ..
        } = node;
        state.erased_update(Hook {
            anc: &mut *anc,
            reg: self,
            rt: &mut *rt,
            id: *id,
        });
        let mut frame = Frame {
            head: &mut **state,
            tail: anc,
        };
        for child in regions.iter_mut() {
            self.update_node(rt, child, &mut frame);
        }
    }

    /// Post-order `on_exit`: descendants first (regions in declaration order), then `node`.
    fn exit_node(&self, rt: &mut Runtime, node: &mut Node, anc: &mut dyn Ancestors) {
        let Node {
            state,
            regions,
            idx,
            id,
        } = node;
        {
            let mut frame = Frame {
                head: &mut **state,
                tail: &mut *anc,
            };
            for child in regions.iter_mut() {
                self.exit_node(rt, child, &mut frame);
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

    /// Dispatches a trigger below (and including) `node`. Returns `true` when every active
    /// branch through `node` was handled at or below it.
    pub(crate) fn dispatch(
        &self,
        rt: &mut Runtime,
        node: &mut Node,
        anc: &mut dyn Ancestors,
        trigger: &mut Dispatch<'_>,
    ) -> bool {
        let Node {
            state,
            regions,
            idx,
            id,
        } = node;
        // A leaf's own branch is unhandled until the leaf handles it.
        let mut unhandled_below = regions.is_empty();
        {
            let mut frame = Frame {
                head: &mut **state,
                tail: &mut *anc,
            };
            for child in regions.iter_mut() {
                if !self.dispatch(rt, child, &mut frame, trigger) {
                    unhandled_below = true;
                }
            }
        }
        if !unhandled_below {
            return true;
        }
        // Runs at most once, on behalf of every branch below that bubbled up unhandled.
        match self.handler(*idx, trigger.event_ty) {
            Some(handler) => {
                let hook = Hook {
                    anc,
                    reg: self,
                    rt,
                    id: *id,
                };
                handler(&mut **state, trigger.event, &mut *trigger.outputs, hook);
                true
            }
            None => false,
        }
    }
}
