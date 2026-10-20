//! Type-erased internals: the static registry built from the `define` functions, the runtime
//! bookkeeping, the active-configuration tree, the ancestor frames, and the traversals.

use crate::context::{Context, ExitContext, Hook};
use crate::definition::Definition;
use crate::state::{NoParent, State};
use crate::trigger::{Event, Trigger};
use std::any::{Any, TypeId, type_name};
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::panic::Location;

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
    /// The requests of the batch being applied. Kept between batches (empty) so that applying
    /// transitions does not allocate once the buffer has grown to its working size.
    queue: VecDeque<Request>,
    /// Active state types and their instance ids. A state is live from just before its
    /// `on_enter` until just after its `on_exit`.
    live: HashMap<TypeId, u64>,
    next_id: u64,
    /// Set while a restart runs the old root's own `on_exit`: the new root is already in the
    /// tree (and live), but its `on_enter` has not run. If that `on_exit` panics, the flag stays
    /// set and the next operation enters the new root before anything else runs on it
    /// ([`Registry::enter_pending_root`]).
    root_pending_enter: bool,
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
        // Each state type appears once in a chart, but not each instance: during a restart the
        // new root is live (same type, new id) before the old root's `on_exit`, and the old
        // root must not remove the new root's entry.
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

/// Runs one state's handler for one trigger type and puts the output into a
/// [`Sink<E::Output>`](Sink).
pub(crate) type HandlerFn = fn(&mut dyn ErasedState, &dyn Any, &mut dyn Any, Hook<'_>);

/// Where a child was listed in its parent's `define`.
pub(crate) type ListedAt = &'static Location<'static>;

/// Registers a child type (monomorphised `register::<C>`); returns its registry index.
pub(crate) type RegisterFn =
    fn(&mut Registry, Option<usize>, usize, Option<Ctor>, Option<ListedAt>) -> usize;

/// Upper bound on transitions applied in one batch; catches `on_enter` hooks that keep
/// requesting transitions in a cycle.
pub(crate) const MAX_TRANSITIONS_PER_BATCH: usize = 10_000;

struct StateInfo {
    ty: TypeId,
    name: &'static str,
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
/// `listed_at` is where the parent listed `S` (`None` for the root).
pub(crate) fn register<S: State>(
    reg: &mut Registry,
    parent: Option<usize>,
    region_in_parent: usize,
    ctor: Option<Ctor>,
    listed_at: Option<ListedAt>,
) -> usize {
    let ty = TypeId::of::<S>();
    // Spec item 11, always on (user decision D1). `child::<C>()` only compiles when
    // `C::Parent == Self`, so the only way to get here twice is a parent listing the same child
    // twice (in one region or in two), which `Definition` already reports at the offending
    // line. This is the backstop: one hash lookup per state, once per `Statechart::new`.
    assert!(
        !reg.index.contains_key(&ty),
        "State type `{}` appears more than once in the statechart hierarchy",
        type_name::<S>()
    );

    let mut def = Definition::<S>::new();
    S::define(&mut def);
    let (regions, triggers) = def.into_parts();

    if parent.is_some() && ctor.is_none() && !regions.is_empty() {
        panic!(
            "State `{0}` has regions but was registered with `child_explicit`: a composite state \
             can be entered on the way to one of its descendants, so it needs a constructor; use \
             `child::<{0}>()` (requires `Default`) or `child_with(...)`; it is registered at {1}",
            type_name::<S>(),
            listed_at.expect("every child is listed somewhere"),
        );
    }

    let idx = reg.states.len();
    let path = match parent {
        Some(p) => reg.states[p].path.iter().copied().chain([idx]).collect(),
        None => Box::from([idx]),
    };
    reg.states.push(StateInfo {
        ty,
        name: type_name::<S>(),
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
            let child_idx =
                (child.register)(reg, Some(idx), region, child.ctor, Some(child.listed_at));
            initial.get_or_insert(child_idx);
        }
        let initial = initial.expect("a region is declared by naming its initial child");
        reg.states[idx].regions.push(initial);
    }
    idx
}

/// Collects the outputs of a trigger's handlers: `trigger` keeps the first one (without
/// allocating), `trigger_all` every one. Every eligible handler runs either way.
pub(crate) enum Sink<O> {
    First(Option<O>),
    All(Vec<O>),
}

impl<O> Sink<O> {
    fn put(&mut self, output: O) {
        match self {
            Sink::First(first) => {
                first.get_or_insert(output);
            }
            Sink::All(all) => all.push(output),
        }
    }
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
        .downcast_mut::<Sink<E::Output>>()
        .expect("output type mismatch");
    outputs.put(state.handle(event, &mut Context::new(hook)));
}

/// A trigger being dispatched.
pub(crate) struct Dispatch<'e> {
    pub(crate) event_ty: TypeId,
    pub(crate) event: &'e dyn Any,
    /// A `Sink<E::Output>`.
    pub(crate) outputs: &'e mut dyn Any,
}

// ---------------------------------------------------------------------------------------------
// Active configuration
// ---------------------------------------------------------------------------------------------

/// An active state and the active child of each of its regions (`regions[i]` is region `i`).
///
/// A region whose state is being exited or entered holds a [vacant](Node::vacant) node while
/// that happens. It is never seen by a hook, and only stays in the tree if a hook (or a
/// constructor) panics in the middle of a transition: the region then has no active state
/// until a later transition enters one.
pub(crate) struct Node {
    pub(crate) state: Box<dyn ErasedState>,
    /// Registry index of the state's type ([`VACANT`] for a vacant node).
    idx: usize,
    /// Instance id.
    id: u64,
    regions: Vec<Node>,
}

/// The `idx` of a vacant node.
const VACANT: usize = usize::MAX;

/// The state of a vacant node. Never entered, exited, updated or dispatched to.
struct Vacant;

impl Debug for Vacant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(empty)")
    }
}

impl State for Vacant {
    type Parent = NoParent;
}

impl Drop for Node {
    /// Drops the descendants first, in post-order (each region's whole subtree, children before
    /// their parent, regions in declaration order: the order `on_exit` runs in), then the state
    /// itself.
    fn drop(&mut self) {
        self.regions.clear();
    }
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
    /// A region with no active state. Allocates nothing.
    fn vacant() -> Self {
        Node {
            state: Box::new(Vacant),
            idx: VACANT,
            id: 0,
            regions: Vec::new(),
        }
    }

    fn is_vacant(&self) -> bool {
        self.idx == VACANT
    }

    /// Post-order (deepest first within a branch, regions in declaration order), excluding self.
    pub(crate) fn collect_descendant_types(&self, out: &mut Vec<TypeId>) {
        for child in self.regions.iter().filter(|child| !child.is_vacant()) {
            child.collect_descendant_types(out);
            out.push(child.state.as_any().type_id());
        }
    }
}

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

/// Re-synchronises the live set with the tree if a batch is interrupted by a panic in a hook.
///
/// Every state in the tree has been entered (at least partly) and not exited, because a state
/// is placed in the tree when it is entered and detached before it is exited. The one exception
/// is a new root whose predecessor's `on_exit` panicked during a restart: it is in the tree but
/// not entered yet, and [`Runtime::root_pending_enter`] says so. A state whose exit was
/// interrupted, or a detached descendant that was never exited, may still be in the live set.
/// Rebuilding the live set from the tree runs no user code, so it is safe during unwinding.
///
/// It also drops the requests that `on_enter` hooks made but the batch never queued (the
/// queued ones go with the batch's own queue), so their targets, and whatever resources they
/// hold, are released while the panic unwinds instead of at the next operation.
struct ResyncOnUnwind<'a> {
    reg: &'a Registry,
    rt: &'a mut Runtime,
    root: &'a mut Node,
    armed: bool,
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

/// Drops the requests recorded by a traversal (`update` or a trigger dispatch) if a hook panics
/// during it. They would never be applied anyway (every operation first discards leftovers),
/// but their targets may hold resources, which are released while the panic unwinds.
struct DiscardRequestsOnUnwind<'a> {
    rt: &'a mut Runtime,
    armed: bool,
}

impl Drop for DiscardRequestsOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.rt.requests.clear();
        }
    }
}

impl Registry {
    /// Registers the whole hierarchy below the root `R`.
    pub(crate) fn build<R: State>() -> Self {
        let mut reg = Registry::default();
        register::<R>(&mut reg, None, 0, None, None);
        reg
    }

    /// The type name of a registered state, for panic messages.
    fn name_of(&self, ty: TypeId) -> &'static str {
        self.index_of(ty).map_or("?", |idx| self.states[idx].name)
    }

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

    /// Mutable version of [`find_active`](Self::find_active).
    pub(crate) fn find_active_mut<'n>(
        &self,
        root: &'n mut Node,
        ty: TypeId,
    ) -> Option<&'n mut Node> {
        let path = &self.states[self.index_of(ty)?].path;
        let (first, rest) = path.split_first()?;
        if root.idx != *first {
            return None;
        }
        let mut node = root;
        for &idx in rest {
            let region = self.states[idx].region_in_parent;
            if node.regions[region].idx != idx {
                return None;
            }
            node = &mut node.regions[region];
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
        let mut node = self.place(rt, 0, root);
        self.enter_placed(rt, &mut Nil, &mut node, Towards::nowhere(&mut None));
        node
    }

    /// A new node for `state` (of registry index `idx`), live from now on, with every region
    /// vacant.
    fn place(&self, rt: &mut Runtime, idx: usize, state: Box<dyn ErasedState>) -> Node {
        let info = &self.states[idx];
        Node {
            state,
            idx,
            id: rt.activate(info.ty),
            regions: info.regions.iter().map(|_| Node::vacant()).collect(),
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

    /// Applies every queued request in order, plus any requested by `on_enter` hooks while
    /// applying them (appended to the end of the queue).
    ///
    /// If a hook panics, the panic propagates and the rest of the batch is dropped; the tree
    /// stays consistent with the live set (see [`ResyncOnUnwind`]).
    pub(crate) fn run_batch(&self, rt: &mut Runtime, root: &mut Node) {
        if rt.requests.is_empty() {
            return;
        }
        let mut guard = ResyncOnUnwind {
            reg: self,
            rt,
            root,
            armed: true,
        };
        self.apply_all(guard.rt, guard.root);
        guard.armed = false;
    }

    /// Finishes a restart that a panic in the old root's own `on_exit` interrupted: the new
    /// root is in the tree but not entered yet. Enters it (its `on_enter`, then its initial
    /// descendants) and applies the transitions their `on_enter` hooks request, as a batch of
    /// its own. Does nothing otherwise. Every public operation calls this first, so `update`,
    /// trigger handlers and `on_exit` never run on a root whose `on_enter` has not run.
    pub(crate) fn enter_pending_root(&self, rt: &mut Runtime, root: &mut Node) {
        if !rt.root_pending_enter {
            return;
        }
        rt.requests.clear();
        let mut guard = ResyncOnUnwind {
            reg: self,
            rt,
            root,
            armed: true,
        };
        self.enter_placed_root(guard.rt, guard.root);
        self.apply_all(guard.rt, guard.root);
        guard.armed = false;
    }

    /// Enters the root, which is already in the tree. From here on it counts as entered, even
    /// if its `on_enter` panics.
    fn enter_placed_root(&self, rt: &mut Runtime, root: &mut Node) {
        rt.root_pending_enter = false;
        self.enter_placed(rt, &mut Nil, root, Towards::nowhere(&mut None));
    }

    fn apply_all(&self, rt: &mut Runtime, root: &mut Node) {
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

        if k == 0 {
            self.restart(rt, root, request.target);
        } else {
            let mut target = Some(request.target);
            self.replace_in(rt, root, &mut Nil, &path[1..k], &path[k..], &mut target);
        }
    }

    /// Exits the whole chart and enters `new_root` with its initial descendants.
    ///
    /// The root slot is never vacant: the new root takes the old one's place once the old
    /// root's descendants are exited, just before the old root's own `on_exit`. Until its own
    /// `on_enter` starts it is marked as not entered yet ([`Runtime::root_pending_enter`]), so
    /// if the old root's `on_exit` panics, the next operation enters it first.
    fn restart(&self, rt: &mut Runtime, root: &mut Node, new_root: Box<dyn ErasedState>) {
        {
            let Node { state, regions, .. } = &mut *root;
            let mut frame = Frame {
                head: &mut **state,
                tail: &mut Nil,
            };
            for slot in regions.iter_mut() {
                self.exit_slot(rt, slot, &mut frame);
            }
        }
        let old = std::mem::replace(root, self.place(rt, 0, new_root));
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

    /// Runs `update` on every active state and records the requested transitions (applied
    /// later by [`run_batch`](Self::run_batch)). A panic drops the requests made so far.
    pub(crate) fn update_root(&self, rt: &mut Runtime, root: &mut Node) {
        rt.requests.clear();
        let mut guard = DiscardRequestsOnUnwind { rt, armed: true };
        self.update_node(guard.rt, root, &mut Nil);
        guard.armed = false;
    }

    /// Runs every eligible handler of a trigger and records the requested transitions
    /// (applied later by [`run_batch`](Self::run_batch)). A panic drops the requests made so
    /// far.
    pub(crate) fn dispatch_root(
        &self,
        rt: &mut Runtime,
        root: &mut Node,
        trigger: &mut Dispatch<'_>,
    ) {
        rt.requests.clear();
        let mut guard = DiscardRequestsOnUnwind { rt, armed: true };
        self.dispatch(guard.rt, root, &mut Nil, trigger);
        guard.armed = false;
    }

    /// Pre-order `update`: parent before children, regions in declaration order.
    fn update_node(&self, rt: &mut Runtime, node: &mut Node, anc: &mut dyn Ancestors) {
        if node.is_vacant() {
            return;
        }
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

    /// Dispatches a trigger below (and including) `node`. Returns `true` when every active
    /// branch through `node` was handled at or below it.
    fn dispatch(
        &self,
        rt: &mut Runtime,
        node: &mut Node,
        anc: &mut dyn Ancestors,
        trigger: &mut Dispatch<'_>,
    ) -> bool {
        // An empty region has no state to handle anything: its branch bubbles up.
        if node.is_vacant() {
            return false;
        }
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
