//! Runtime bookkeeping: the live set, instance ids and transition requests.

use super::ErasedState;
use std::any::TypeId;
use std::collections::{HashMap, VecDeque};

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
    pub(super) queue: VecDeque<Request>,
    /// Active state types and their instance ids. A state is live from just before its
    /// `on_enter` until just after its `on_exit`.
    pub(super) live: HashMap<TypeId, u64>,
    next_id: u64,
    /// Set while a restart runs the old root's own `on_exit`: the new root is already in the
    /// tree (and live), but its `on_enter` has not run. If that `on_exit` panics, the flag stays
    /// set and the next operation enters the new root before anything else runs on it
    /// ([`Engine::enter_pending_root`](super::Engine::enter_pending_root)).
    pub(super) root_pending_enter: bool,
}

impl Runtime {
    pub(crate) fn is_live(&self, ty: TypeId) -> bool {
        self.live.contains_key(&ty)
    }

    pub(super) fn is_live_instance(&self, ty: TypeId, id: u64) -> bool {
        self.live.get(&ty) == Some(&id)
    }

    pub(super) fn activate(&mut self, ty: TypeId) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.live.insert(ty, id);
        id
    }

    pub(super) fn deactivate(&mut self, ty: TypeId, id: u64) {
        // Each state type appears once in a chart, but not each instance: during a restart the
        // new root is live (same type, new id) before the old root's `on_exit`, and the old
        // root must not remove the new root's entry.
        if self.is_live_instance(ty, id) {
            self.live.remove(&ty);
        }
    }
}
