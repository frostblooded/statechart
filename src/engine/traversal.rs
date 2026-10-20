//! The traversals that run user hooks without changing the tree: `update` and trigger dispatch.
//! Both only record transition requests, which [`Engine`](super::Engine) applies afterwards.

use super::{Ancestors, ErasedState, Frame, Node, Registry, Runtime};
use crate::context::{Context, Hook};
use crate::trigger::{Event, Trigger};
use std::any::{Any, TypeId};

/// Runs one state's handler for one trigger type and puts the output into a
/// [`Sink<E::Output>`](Sink).
pub(crate) type HandlerFn = fn(&mut dyn ErasedState, &dyn Any, &mut dyn Any, Hook<'_>);

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
pub(super) struct Dispatch<'e> {
    pub(super) event_ty: TypeId,
    pub(super) event: &'e dyn Any,
    /// A `Sink<E::Output>`.
    pub(super) outputs: &'e mut dyn Any,
}

impl Registry {
    fn handler(&self, idx: usize, event: TypeId) -> Option<HandlerFn> {
        self.states[idx]
            .triggers
            .iter()
            .find(|(e, _)| *e == event)
            .map(|(_, h)| *h)
    }

    /// Pre-order `update`: parent before children, regions in declaration order.
    pub(super) fn update_node(&self, rt: &mut Runtime, node: &mut Node, anc: &mut dyn Ancestors) {
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

    /// Dispatches a trigger below (and including) `node`. Returns `true` when every active
    /// branch through `node` was handled at or below it.
    pub(super) fn dispatch(
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
