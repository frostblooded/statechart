//! The registry: static knowledge of the hierarchy, built once from the `define` functions.

use super::{ErasedState, HandlerFn};
use crate::definition::Definition;
use crate::state::State;
use std::any::{TypeId, type_name};
use std::collections::HashMap;
use std::panic::Location;

/// Builds a child from its (already entered) parent.
pub(crate) type Ctor = Box<dyn Fn(&dyn ErasedState) -> Box<dyn ErasedState>>;

/// Where a child was listed in its parent's `define`.
pub(crate) type ListedAt = &'static Location<'static>;

/// Registers a child type (monomorphised `register::<C>`); returns its registry index.
pub(crate) type RegisterFn =
    fn(&mut Registry, Option<usize>, usize, Option<Ctor>, Option<ListedAt>) -> usize;

pub(super) struct StateInfo {
    pub(super) ty: TypeId,
    pub(super) name: &'static str,
    /// Index of this state's region in its parent.
    pub(super) region_in_parent: usize,
    /// Registry index of each region's initial child.
    pub(super) regions: Vec<usize>,
    pub(super) triggers: Vec<(TypeId, HandlerFn)>,
    /// `None` only for the root and for leaves registered with `child_explicit`.
    pub(super) ctor: Option<Ctor>,
    /// Registry indices from the root down to this state (inclusive).
    pub(super) path: Box<[usize]>,
}

/// Every state type of one chart, registered once by [`Registry::build`]. The root is index 0.
#[derive(Default)]
pub(crate) struct Registry {
    pub(super) states: Vec<StateInfo>,
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

impl Registry {
    /// Registers the whole hierarchy below the root `R`.
    pub(super) fn build<R: State>() -> Self {
        let mut reg = Registry::default();
        register::<R>(&mut reg, None, 0, None, None);
        reg
    }

    pub(crate) fn index_of(&self, ty: TypeId) -> Option<usize> {
        self.index.get(&ty).copied()
    }

    /// The type name of a registered state, for panic messages.
    pub(super) fn name_of(&self, ty: TypeId) -> &'static str {
        self.index_of(ty).map_or("?", |idx| self.states[idx].name)
    }
}
