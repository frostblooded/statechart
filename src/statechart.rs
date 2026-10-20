//! The [`Statechart`]: owns the active configuration and runs updates, triggers and transitions.

use crate::engine::{Dispatch, Nil, Node, Registry, Runtime, Sink};
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
/// [`is_active`](Self::is_active), [`state`](Self::state) / [`state_mut`](Self::state_mut),
/// [`root`](Self::root) / [`root_mut`](Self::root_mut),
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
///
/// # When states are dropped
///
/// A state's struct is dropped right after its `on_exit`, deepest first, before any state of
/// the same transition is entered. So a state that holds a resource (a socket, a file lock, an
/// audio handle) releases it before the state that replaces it, or the new instance of an
/// external self-transition, runs its `on_enter`. Dropping the `Statechart` drops every active
/// state, deepest first, **without** running `on_exit`.
///
/// # Panics in hooks
///
/// A panic in a hook (or in a constructor given to `region_with` / `child_with`) propagates to
/// the caller of [`new`](Self::new), [`update`](Self::update), [`trigger`](Self::trigger) or
/// [`trigger_all`](Self::trigger_all). If the caller catches it, the chart stays consistent and
/// usable:
///
/// * a panic in `update` or in a trigger handler changes nothing: the transitions requested so
///   far are discarded;
/// * a panic while a transition is applied stops that transition (and the rest of the batch)
///   where it was. States already exited stay exited, and the rest of the subtree that was
///   being exited is dropped without `on_exit`. A state whose `on_enter` panicked counts as
///   entered; states not entered yet are not entered. A region left without a new state is
///   **empty**: it has no active state (printed as `(empty)`), updates skip it, a trigger
///   bubbles from it straight to its parent, and the next transition to one of its states (or
///   to an ancestor) enters it again. A restart (a transition to the root type) never leaves
///   the chart without a root: the old root stays until its descendants are exited, then the
///   new root takes its place.
/// * If that old root's own `on_exit` panics, the new root is already in place but **not
///   entered yet**: it is the active root ([`root`](Self::root) returns it,
///   [`is_active`](Self::is_active) is `true`), its regions are empty, and its `on_enter` has
///   not run. The next [`update`](Self::update), [`trigger`](Self::trigger) or
///   [`trigger_all`](Self::trigger_all) first enters it (its `on_enter`, its initial
///   descendants, then the transitions their `on_enter` hooks request) and only then does its
///   own work. So `update`, trigger handlers and `on_exit` never run on a state whose
///   `on_enter` has not run.
///
/// Either way [`is_active`](Self::is_active), [`state`](Self::state) and the `Debug` output
/// agree, no state's `on_exit` runs twice, and requests made by the remaining states are
/// applied as usual.
///
/// # Limitations
///
/// * `Statechart` is neither `Send` nor `Sync`, whatever the states are: it stores states as
///   `Box<dyn ...>` without a `Send` bound. Keep a chart on the thread that created it.
/// * Hooks cannot fire triggers (there are no internal events); they request transitions.
/// * States are `'static` (they cannot borrow), because they are stored as `dyn Any`.
/// * Triggers and their outputs are `'static` too (a trigger cannot carry `&str` or
///   `&mut World`), because they are passed to the handlers type-erased, and `update` takes no
///   input. Pass per-call data as owned trigger fields, or write it into the root with
///   [`root_mut`](Self::root_mut) before [`update`](Self::update) and read it in the hooks with
///   [`ctx.ancestor()`](crate::context::Context::ancestor). See
///   [`Event`](crate::trigger::Event#triggers-cannot-borrow).
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
    /// * if a state type appears more than once in the hierarchy (a parent lists the same child
    ///   twice), with the message ``State type `X` appears more than once in the statechart
    ///   hierarchy: `P` lists it twice in `define` ``, reported at the second listing;
    /// * if a state with regions is registered with
    ///   [`child_explicit`](crate::definition::Region::child_explicit);
    /// * if a state registers the same trigger twice;
    /// * if the `on_enter` hooks keep requesting transitions in a cycle (more than 10 000
    ///   transitions in one batch).
    ///
    /// A panic in a hook propagates (see [Panics in hooks](Self#panics-in-hooks)).
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
    ///
    /// # Panics
    ///
    /// If the `on_enter` hooks keep requesting transitions in a cycle (more than 10 000
    /// transitions in one batch), or if a hook requests a transition to a state its parent does
    /// not list ([`Context::transition`](crate::context::Context::transition)). A panic in a hook
    /// propagates (see [Panics in hooks](Self#panics-in-hooks)).
    pub fn update(&mut self) {
        self.registry
            .enter_pending_root(&mut self.runtime, &mut self.root);
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
    ///
    /// Only types implementing [`Event`] are triggers:
    ///
    /// ```compile_fail,E0277
    /// # use statechart::prelude::*;
    /// #[derive(Debug)] struct Timer;
    /// impl State for Timer { type Parent = NoParent; }
    /// struct NotEvent;
    /// let mut chart = Statechart::new(Timer);
    /// chart.trigger(NotEvent); // error: `NotEvent` is not a trigger
    /// ```
    ///
    /// # Panics
    ///
    /// Like [`update`](Self::update), when the transitions requested by the handlers are
    /// applied.
    pub fn trigger<E: Event>(&mut self, trigger: E) -> Result<E::Output, NotHandled> {
        match self.dispatch(trigger, Sink::First(None)) {
            Sink::First(Some(output)) => Ok(output),
            _ => Err(NotHandled),
        }
    }

    /// Fires a trigger and returns the outputs of every handler that ran, in the order they ran:
    /// a state's handler runs after the handlers below it, regions in declaration order. The
    /// vector is never empty: no handler at all is `Err(NotHandled)`.
    ///
    /// Transitions requested by the handlers are applied after the whole dispatch.
    ///
    /// # Panics
    ///
    /// Like [`update`](Self::update), when the transitions requested by the handlers are
    /// applied.
    pub fn trigger_all<E: Event>(&mut self, trigger: E) -> Result<Vec<E::Output>, NotHandled> {
        match self.dispatch(trigger, Sink::All(Vec::new())) {
            Sink::All(outputs) if !outputs.is_empty() => Ok(outputs),
            _ => Err(NotHandled),
        }
    }

    /// Runs every eligible handler of `trigger` into `sink`, then applies the requested
    /// transitions.
    fn dispatch<E: Event>(&mut self, trigger: E, mut sink: Sink<E::Output>) -> Sink<E::Output> {
        self.registry
            .enter_pending_root(&mut self.runtime, &mut self.root);
        self.runtime.requests.clear();
        self.registry.dispatch(
            &mut self.runtime,
            &mut self.root,
            &mut Nil,
            &mut Dispatch {
                event_ty: TypeId::of::<E>(),
                event: &trigger,
                outputs: &mut sink,
            },
        );
        self.registry.run_batch(&mut self.runtime, &mut self.root);
        sink
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

    /// The data of the state `T`, if it is active. `T` must belong to this chart:
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
    /// chart.state::<Door_Open>(); // error: `Door_Open` belongs to the `Door` chart
    /// ```
    pub fn state<T: InChart<R>>(&self) -> Option<&T> {
        self.registry
            .find_active(&self.root, TypeId::of::<T>())
            .and_then(|node| node.state.as_any().downcast_ref())
    }

    /// The data of the state `T`, mutably, if it is active. Changing a state's data never
    /// changes which states are active. `T` must belong to this chart:
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
    /// let mut chart = Statechart::new(Timer);
    /// chart.state_mut::<Door_Open>(); // error: `Door_Open` belongs to the `Door` chart
    /// ```
    pub fn state_mut<T: InChart<R>>(&mut self) -> Option<&mut T> {
        self.registry
            .find_active_mut(&mut self.root, TypeId::of::<T>())
            .and_then(|node| node.state.as_any_mut().downcast_mut())
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
    /// `format!("{chart:?}")`. A region left without an active state by a panicking hook prints
    /// as `(empty)` (see [Panics in hooks](Self#panics-in-hooks)).
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
