//! Type-erased internals. [`Engine`] is the one entry point `Statechart` uses: it owns the
//! static registry, the runtime bookkeeping and the active-configuration tree, and runs every
//! operation as the panic-safety rules need it (pending root first, then the traversal, then
//! the batch of transitions, each step resynchronised if a hook panics).
//!
//! * [`registry`]: static knowledge of the hierarchy, built from the `define` functions;
//! * [`runtime`]: the live set, instance ids and transition requests;
//! * [`tree`]: the active configuration and lookups in it;
//! * [`ancestors`]: the stack frames that give hooks their ancestors;
//! * [`traversal`]: `update` and trigger dispatch, which only record requests;
//! * [`transition`]: entering and exiting states, applying a batch of requests.

mod ancestors;
mod erased;
mod registry;
mod runtime;
mod transition;
mod traversal;
mod tree;

pub(crate) use ancestors::Ancestors;
use ancestors::{Frame, Nil};
pub(crate) use erased::ErasedState;
pub(crate) use registry::{Ctor, ListedAt, RegisterFn, Registry, register};
pub(crate) use runtime::{Request, Runtime};
use transition::ResyncOnUnwind;
use traversal::Dispatch;
pub(crate) use traversal::{HandlerFn, Sink, call_handler};
use tree::Node;

use crate::state::State;
use crate::trigger::Event;
use std::any::TypeId;
use std::fmt::{self, Debug};

/// A running chart, type-erased: everything `Statechart<R>` does goes through here.
pub(crate) struct Engine {
    registry: Registry,
    runtime: Runtime,
    root: Node,
}

impl Engine {
    /// Registers the hierarchy below `R`, enters `root` and its initial descendants, then
    /// applies the transitions their `on_enter` hooks requested.
    pub(crate) fn new<R: State>(root: R) -> Self {
        let registry = Registry::build::<R>();
        let mut runtime = Runtime::default();
        let root = registry.enter_root(&mut runtime, Box::new(root));
        let mut engine = Engine {
            registry,
            runtime,
            root,
        };
        engine.run_batch();
        engine
    }

    /// Runs `update` on every active state, then applies the requested transitions.
    pub(crate) fn update(&mut self) {
        self.enter_pending_root();
        self.traverse(|reg, rt, root| reg.update_node(rt, root, &mut Nil));
        self.run_batch();
    }

    /// Runs every eligible handler of `event` into `sink`, then applies the requested
    /// transitions.
    pub(crate) fn dispatch<E: Event>(&mut self, event: &E, sink: &mut Sink<E::Output>) {
        self.enter_pending_root();
        let mut trigger = Dispatch {
            event_ty: TypeId::of::<E>(),
            event,
            outputs: sink,
        };
        self.traverse(|reg, rt, root| {
            reg.dispatch(rt, root, &mut Nil, &mut trigger);
        });
        self.run_batch();
    }

    /// Runs one step of an operation. If a hook panics during it, the live set is rebuilt from
    /// the tree and the pending requests are dropped while the panic unwinds
    /// ([`ResyncOnUnwind`]).
    fn guarded(&mut self, step: impl FnOnce(&Registry, &mut Runtime, &mut Node)) {
        let Engine {
            registry,
            runtime,
            root,
        } = self;
        let mut guard = ResyncOnUnwind {
            reg: registry,
            rt: runtime,
            root,
            armed: true,
        };
        step(guard.reg, guard.rt, guard.root);
        guard.armed = false;
    }

    /// A traversal (`update` or a trigger dispatch): records requests from a clean slate,
    /// applies nothing.
    fn traverse(&mut self, step: impl FnOnce(&Registry, &mut Runtime, &mut Node)) {
        self.runtime.requests.clear();
        self.guarded(step);
    }

    /// Applies the recorded requests as one batch, plus those that `on_enter` hooks request
    /// while it runs.
    fn run_batch(&mut self) {
        if self.runtime.requests.is_empty() {
            return;
        }
        self.guarded(|reg, rt, root| reg.apply_all(rt, root));
    }

    /// Finishes a restart that a panic in the old root's own `on_exit` interrupted: the new
    /// root is in the tree but not entered yet. Enters it (its `on_enter`, then its initial
    /// descendants) and applies the transitions their `on_enter` hooks request, as a batch of
    /// its own. Does nothing otherwise. Every operation calls this first, so `update`, trigger
    /// handlers and `on_exit` never run on a root whose `on_enter` has not run.
    fn enter_pending_root(&mut self) {
        if !self.runtime.root_pending_enter {
            return;
        }
        self.runtime.requests.clear();
        self.guarded(|reg, rt, root| {
            reg.enter_placed_root(rt, root);
            reg.apply_all(rt, root);
        });
    }

    pub(crate) fn is_live(&self, ty: TypeId) -> bool {
        self.runtime.is_live(ty)
    }

    /// The active state of type `ty`, if any.
    pub(crate) fn state(&self, ty: TypeId) -> Option<&dyn ErasedState> {
        let node = self.registry.find_active(&self.root, ty)?;
        Some(&*node.state)
    }

    /// Mutable version of [`state`](Self::state).
    pub(crate) fn state_mut(&mut self, ty: TypeId) -> Option<&mut dyn ErasedState> {
        let node = self.registry.find_active_mut(&mut self.root, ty)?;
        Some(&mut *node.state)
    }

    pub(crate) fn root(&self) -> &dyn ErasedState {
        &*self.root.state
    }

    pub(crate) fn root_mut(&mut self) -> &mut dyn ErasedState {
        &mut *self.root.state
    }

    /// The active states except the root, in post-order.
    pub(crate) fn active_descendant_types(&self) -> Vec<TypeId> {
        let mut out = Vec::new();
        self.root.collect_descendant_types(&mut out);
        out
    }
}

impl Debug for Engine {
    /// The active configuration, e.g. `"A -> [A_B -> A_B_X, A_C]"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.root)
    }
}

#[cfg(test)]
mod tests;
