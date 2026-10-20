//! Erased states: the object-safe view of [`State`] that the tree stores.

use crate::context::{Context, ExitContext, Hook};
use crate::state::State;
use std::any::Any;
use std::fmt::Debug;

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
