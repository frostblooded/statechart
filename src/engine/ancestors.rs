//! Ancestor frames: how a hook reaches its ancestors' data without `unsafe`.

use super::ErasedState;
use std::any::TypeId;

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
pub(super) struct Nil;

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
pub(super) struct Frame<'a> {
    pub(super) head: &'a mut dyn ErasedState,
    pub(super) tail: &'a mut dyn Ancestors,
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
