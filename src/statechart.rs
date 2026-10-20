//! The [`Statechart`]: owns the active configuration and runs updates, triggers and transitions.

use crate::engine::{Dispatch, Nil, Node, Registry, Runtime};
use crate::state::{InChart, NoParent, State};
use crate::trigger::{Event, NotHandled};
use std::any::TypeId;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

/// A running statechart whose root state is `R`.
///
/// The chart owns the active configuration: the root and, for each region of each active
/// state, its one active child. It is driven from outside with [`update`](Self::update),
/// [`trigger`](Self::trigger) and [`trigger_all`](Self::trigger_all), and inspected with
/// [`is_active`](Self::is_active), [`state`](Self::state), [`root`](Self::root),
/// [`get_debug_name`](Self::get_debug_name) (or `{:?}`) and
/// [`get_recursive_active_type_ids`](Self::get_recursive_active_type_ids).
///
/// ```
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// #[derive(Debug)]
/// struct Door;
/// impl State for Door {
///     type Parent = NoParent;
///     fn define(def: &mut Definition<Self>) {
///         def.region::<Door_Closed>().child::<Door_Open>();
///     }
/// }
///
/// #[derive(Debug, Default)]
/// struct Door_Closed;
/// impl State for Door_Closed {
///     type Parent = Door;
///     fn update(&mut self, ctx: &mut Context<Self>) {
///         ctx.transition(Door_Open);
///     }
/// }
///
/// #[derive(Debug, Default)]
/// struct Door_Open;
/// impl State for Door_Open {
///     type Parent = Door;
/// }
///
/// let mut chart = Statechart::new(Door);
/// assert!(chart.is_active::<Door_Closed>());
/// chart.update();
/// assert_eq!(format!("{chart:?}"), "Door -> Door_Open");
/// ```
///
/// Only a root (a state with `type Parent = NoParent`) can start a chart:
///
/// ```compile_fail,E0271
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// # #[derive(Debug)] struct Timer;
/// # impl State for Timer { type Parent = NoParent; fn define(d: &mut Definition<Self>) { d.region::<Timer_Elapsed>(); } }
/// #[derive(Debug, Default)]
/// struct Timer_Elapsed;
/// impl State for Timer_Elapsed { type Parent = Timer; }
///
/// Statechart::new(Timer_Elapsed); // error: expected `NoParent`, found `Timer`
/// ```
pub struct Statechart<R> {
    root: Node,
    registry: Registry,
    runtime: Runtime,
    _root: PhantomData<fn() -> R>,
}

impl<R: State<Parent = NoParent>> Statechart<R> {
    /// Builds the chart: registers the hierarchy declared by the `define` functions, then
    /// enters `root` and its initial descendants (`on_enter` parent before child, regions in
    /// declaration order). Transitions requested by those `on_enter` hooks are applied before
    /// `new` returns.
    ///
    /// # Panics
    ///
    /// These checks run in debug and release builds alike:
    ///
    /// * if a state type appears more than once in the hierarchy (for example, a parent lists
    ///   the same child twice), with the message
    ///   ``State type `X` appears more than once in the statechart hierarchy``;
    /// * if a state with regions is registered with
    ///   [`child_explicit`](crate::definition::Region::child_explicit);
    /// * if a state registers the same trigger twice.
    pub fn new(root: R) -> Self {
        let registry = Registry::build::<R>();
        let mut runtime = Runtime::default();
        let mut root = registry.enter_root(&mut runtime, Box::new(root));
        registry.run_batch(&mut runtime, &mut root);
        Statechart {
            root,
            registry,
            runtime,
            _root: PhantomData,
        }
    }

    /// Runs `update` on every active state (parent before children, regions in declaration
    /// order), then applies the requested transitions as one batch.
    ///
    /// Nothing changes while the hooks run. Afterwards the requests are applied in request
    /// order; a request is dropped if an earlier transition of the batch exited the state that
    /// made it, and requests made by `on_enter` hooks during the batch are appended to it.
    pub fn update(&mut self) {
        self.runtime.requests.clear();
        self.registry
            .update_node(&mut self.runtime, &mut self.root, &mut Nil);
        self.registry.run_batch(&mut self.runtime, &mut self.root);
    }

    /// Fires a trigger and returns the output of the first handler that ran.
    ///
    /// Each active branch bubbles independently from its leaf towards the root; the deepest
    /// state on the branch that registered `E` handles it, and a shared ancestor runs at most
    /// once. With parallel regions several handlers may run (all of them do); use
    /// [`trigger_all`](Self::trigger_all) to get every output.
    ///
    /// Returns `Err(NotHandled)` if no active state registered `E` with
    /// [`def.trigger::<E>()`](crate::definition::Definition::trigger). A state that implements
    /// [`Trigger<E>`](crate::trigger::Trigger) without registering it does not count.
    pub fn trigger<E: Event>(&mut self, trigger: E) -> Result<E::Output, NotHandled> {
        self.trigger_all(trigger)
            .map(|outputs| outputs.into_iter().next().expect("never empty"))
    }

    /// Fires a trigger and returns the outputs of every handler that ran, in the order they ran:
    /// a state's handler runs after the handlers below it, regions in declaration order. The
    /// vector is never empty: no handler at all is `Err(NotHandled)`.
    ///
    /// Transitions requested by the handlers are applied after the whole dispatch.
    pub fn trigger_all<E: Event>(&mut self, trigger: E) -> Result<Vec<E::Output>, NotHandled> {
        self.runtime.requests.clear();
        let mut outputs: Vec<E::Output> = Vec::new();
        self.registry.dispatch(
            &mut self.runtime,
            &mut self.root,
            &mut Nil,
            &mut Dispatch {
                event_ty: TypeId::of::<E>(),
                event: &trigger,
                outputs: &mut outputs,
            },
        );
        self.registry.run_batch(&mut self.runtime, &mut self.root);
        if outputs.is_empty() {
            Err(NotHandled)
        } else {
            Ok(outputs)
        }
    }

    /// Whether a state of type `T` is active. `T` must belong to this chart (compile-time):
    ///
    /// ```compile_fail,E0277
    /// # #![allow(non_camel_case_types)]
    /// # use statechart::prelude::*;
    /// #[derive(Debug)] struct Timer;
    /// #[derive(Debug)] struct Door;
    /// #[derive(Debug, Default)] struct Door_Open;
    /// impl State for Timer { type Parent = NoParent; }
    /// impl State for Door { type Parent = NoParent; fn define(d: &mut Definition<Self>) { d.region::<Door_Open>(); } }
    /// impl State for Door_Open { type Parent = Door; }
    /// let chart = Statechart::new(Timer);
    /// chart.is_active::<Door_Open>(); // error: `Door_Open` is not a state of the `Timer` chart
    /// ```
    pub fn is_active<T: InChart<R>>(&self) -> bool {
        self.runtime.is_live(TypeId::of::<T>())
    }

    /// The data of the state `T`, if it is active.
    pub fn state<T: InChart<R>>(&self) -> Option<&T> {
        self.registry
            .find_active(&self.root, TypeId::of::<T>())
            .and_then(|node| node.state.as_any().downcast_ref())
    }

    /// The root state's data.
    pub fn root(&self) -> &R {
        self.root
            .state
            .as_any()
            .downcast_ref()
            .expect("the root node holds `R`")
    }

    /// The root state's data, mutably.
    pub fn root_mut(&mut self) -> &mut R {
        self.root
            .state
            .as_any_mut()
            .downcast_mut()
            .expect("the root node holds `R`")
    }

    /// The active configuration: `"A -> A_B"` for a single chain, `"A -> [A_B -> A_B_X, A_C]"`
    /// for a state with parallel regions. Each state prints with its own `Debug`. Same as
    /// `format!("{chart:?}")`.
    pub fn get_debug_name(&self) -> String {
        format!("{self:?}")
    }

    /// `TypeId`s of the active states except the root: deepest first within each branch,
    /// branches in region declaration order (a post-order walk, which is also the order
    /// `on_exit` runs in). For a single chain `A -> A_B -> A_B_C` this is `[A_B_C, A_B]`.
    pub fn get_recursive_active_type_ids(&self) -> Vec<TypeId> {
        let mut out = Vec::new();
        self.root.collect_descendant_types(&mut out);
        out
    }
}

impl<R> Debug for Statechart<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.root)
    }
}
