//! The active configuration: a tree of [`Node`]s, one active child per region.

use super::{ErasedState, Registry};
use crate::state::{NoParent, State};
use std::any::TypeId;
use std::fmt::{self, Debug};

/// An active state and the active child of each of its regions (`regions[i]` is region `i`).
///
/// A region whose state is being exited or entered holds a [vacant](Node::vacant) node while
/// that happens. It is never seen by a hook, and only stays in the tree if a hook (or a
/// constructor) panics in the middle of a transition: the region then has no active state
/// until a later transition enters one.
pub(super) struct Node {
    pub(super) state: Box<dyn ErasedState>,
    /// Registry index of the state's type ([`VACANT`] for a vacant node).
    pub(super) idx: usize,
    /// Instance id.
    pub(super) id: u64,
    pub(super) regions: Vec<Node>,
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
    pub(super) fn vacant() -> Self {
        Node {
            state: Box::new(Vacant),
            idx: VACANT,
            id: 0,
            regions: Vec::new(),
        }
    }

    pub(super) fn is_vacant(&self) -> bool {
        self.idx == VACANT
    }

    /// Post-order (deepest first within a branch, regions in declaration order), excluding self.
    pub(super) fn collect_descendant_types(&self, out: &mut Vec<TypeId>) {
        for child in self.regions.iter().filter(|child| !child.is_vacant()) {
            child.collect_descendant_types(out);
            out.push(child.state.as_any().type_id());
        }
    }
}

impl Registry {
    /// The active child of `node` in the region of `idx`, if it is `idx`.
    fn active_child<'n>(&self, node: &'n Node, idx: usize) -> Option<&'n Node> {
        let child = &node.regions[self.states[idx].region_in_parent];
        (child.idx == idx).then_some(child)
    }

    /// Mutable version of [`active_child`](Self::active_child).
    fn active_child_mut<'n>(&self, node: &'n mut Node, idx: usize) -> Option<&'n mut Node> {
        let child = &mut node.regions[self.states[idx].region_in_parent];
        (child.idx == idx).then_some(child)
    }

    /// How many states of `path` (registry indices from the root down) are active, counted
    /// from the root.
    pub(super) fn active_depth(&self, root: &Node, path: &[usize]) -> usize {
        let Some((&first, rest)) = path.split_first() else {
            return 0;
        };
        if root.idx != first {
            return 0;
        }
        let mut node = root;
        let mut depth = 1;
        for &idx in rest {
            match self.active_child(node, idx) {
                Some(child) => node = child,
                None => break,
            }
            depth += 1;
        }
        depth
    }

    /// Follows the static path of `ty` through the active tree.
    pub(super) fn find_active<'n>(&self, root: &'n Node, ty: TypeId) -> Option<&'n Node> {
        let (&first, rest) = self.states[self.index_of(ty)?].path.split_first()?;
        if root.idx != first {
            return None;
        }
        rest.iter()
            .try_fold(root, |node, &idx| self.active_child(node, idx))
    }

    /// Mutable version of [`find_active`](Self::find_active).
    pub(super) fn find_active_mut<'n>(
        &self,
        root: &'n mut Node,
        ty: TypeId,
    ) -> Option<&'n mut Node> {
        let (&first, rest) = self.states[self.index_of(ty)?].path.split_first()?;
        if root.idx != first {
            return None;
        }
        rest.iter()
            .try_fold(root, |node, &idx| self.active_child_mut(node, idx))
    }
}
