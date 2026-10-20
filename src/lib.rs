//! Hierarchical state machines (statecharts) with parallel regions, typed triggers and a
//! compile-time checked hierarchy. No dependencies, no `unsafe`.
//!
//! # Concepts
//!
//! * A **state** is a plain struct implementing [`State`](state::State); its fields are its
//!   data. Only active states exist: an inactive state's struct is never built.
//! * `type Parent` names the state's parent ([`NoParent`](state::NoParent) for the root). This
//!   one line is the static hierarchy: it types [`parent()`](context::Context::parent), lets
//!   transitions enter ancestors that are not active yet, and makes the compiler reject wrong
//!   parents, foreign transition targets and unimplemented triggers.
//! * [`State::define`](state::State::define) declares the state's **regions** and the triggers
//!   it handles. Each region names an initial child and its other children, and exactly one
//!   child per region is active. A leaf has no regions, a plain composite state has one, and
//!   two or more regions run in parallel.
//! * A **trigger** is a type implementing [`Event`](trigger::Event). A state handles it by
//!   implementing [`Trigger<E>`](trigger::Trigger) *and* registering it with
//!   [`def.trigger::<E>()`](definition::Definition::trigger) in `define`. A trigger bubbles
//!   from each active leaf towards the root, and the deepest state that handles it wins.
//! * A **transition** is requested from a hook with
//!   [`ctx.transition(Target { .. })`](context::Context::transition) and applied once the
//!   current update or trigger dispatch has finished.
//!
//! State structs are named as breadcrumbs of their ancestors: `Timer_Active_Running` is
//! `Running`, a child of `Active`, which is a child of the root `Timer`. Sibling names are never
//! joined: `Timer_Active_Running` and `Timer_Active_Paused` are siblings.
//!
//! # Example: triggers and fallback
//!
//! `A` has two children, `A_B1` and `A_B2`. `A_B1` handles `Foo` and `Bar`; `A_B2` handles
//! neither; `A` handles `Foo` (but not `Bar`), so it is the fallback for `Foo` when `A_B2` is
//! active.
//!
//! ```
//! #![allow(non_camel_case_types)]
//! use statechart::prelude::*;
//!
//! /// Returns the name of the state that handled it.
//! struct Foo;
//! impl Event for Foo {
//!     type Output = &'static str;
//! }
//!
//! /// Like `Foo`, but `A` has no fallback for it.
//! struct Bar;
//! impl Event for Bar {
//!     type Output = &'static str;
//! }
//!
//! /// Switches `A` to `A_B2`.
//! struct Switch;
//! impl Event for Switch {
//!     type Output = ();
//! }
//!
//! #[derive(Debug)]
//! struct A;
//! impl State for A {
//!     type Parent = NoParent;
//!     fn define(def: &mut Definition<Self>) {
//!         def.region::<A_B1>().child::<A_B2>(); // one region: starts in A_B1
//!         def.trigger::<Foo>().trigger::<Switch>();
//!     }
//! }
//! impl Trigger<Foo> for A {
//!     fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
//!         "A"
//!     }
//! }
//! impl Trigger<Switch> for A {
//!     fn handle(&mut self, _: &Switch, ctx: &mut Context<Self>) {
//!         ctx.transition(A_B2);
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! struct A_B1;
//! impl State for A_B1 {
//!     type Parent = A;
//!     fn define(def: &mut Definition<Self>) {
//!         def.trigger::<Foo>().trigger::<Bar>();
//!     }
//! }
//! impl Trigger<Foo> for A_B1 {
//!     fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
//!         "A_B1"
//!     }
//! }
//! impl Trigger<Bar> for A_B1 {
//!     fn handle(&mut self, _: &Bar, _: &mut Context<Self>) -> &'static str {
//!         "A_B1"
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! struct A_B2;
//! impl State for A_B2 {
//!     type Parent = A;
//! }
//!
//! let mut chart = Statechart::new(A);
//! assert_eq!(chart.get_debug_name(), "A -> A_B1");
//! assert_eq!(chart.trigger(Foo), Ok("A_B1")); // only the deepest handler runs
//! assert_eq!(chart.trigger(Bar), Ok("A_B1"));
//!
//! chart.trigger(Switch).unwrap();
//! assert_eq!(chart.get_debug_name(), "A -> A_B2");
//! assert_eq!(chart.trigger(Foo), Ok("A")); // A_B2 does not handle Foo: falls back to A
//! assert_eq!(chart.trigger(Bar), Err(NotHandled)); // nobody in the active chain handles Bar
//! ```
//!
//! # Example: parallel regions and parent data
//!
//! `Robot` has two regions that are active at the same time. Each branch bubbles a trigger on
//! its own, and a shared ancestor runs at most once. A child reads and changes its parent's
//! data with [`ctx.parent_mut()`](context::Context::parent_mut), which is statically typed.
//!
//! ```
//! #![allow(non_camel_case_types)]
//! use statechart::prelude::*;
//!
//! /// Asks the active states what they are doing.
//! struct Status;
//! impl Event for Status {
//!     type Output = &'static str;
//! }
//!
//! /// Starts walking.
//! struct Walk;
//! impl Event for Walk {
//!     type Output = ();
//! }
//!
//! #[derive(Debug, Default)]
//! struct Robot {
//!     steps: u32,
//! }
//! impl State for Robot {
//!     type Parent = NoParent;
//!     fn define(def: &mut Definition<Self>) {
//!         def.region::<Robot_Still>().child::<Robot_Walking>(); // region 0: the legs
//!         def.region::<Robot_Silent>(); // region 1, in parallel: the voice
//!         def.trigger::<Status>().trigger::<Walk>();
//!     }
//! }
//! impl Trigger<Status> for Robot {
//!     fn handle(&mut self, _: &Status, _: &mut Context<Self>) -> &'static str {
//!         "idle"
//!     }
//! }
//! impl Trigger<Walk> for Robot {
//!     fn handle(&mut self, _: &Walk, ctx: &mut Context<Self>) {
//!         ctx.transition(Robot_Walking); // changes region 0 only
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! struct Robot_Still;
//! impl State for Robot_Still {
//!     type Parent = Robot;
//! }
//!
//! #[derive(Debug, Default)]
//! struct Robot_Walking;
//! impl State for Robot_Walking {
//!     type Parent = Robot;
//!     fn define(def: &mut Definition<Self>) {
//!         def.trigger::<Status>();
//!     }
//!     fn update(&mut self, ctx: &mut Context<Self>) {
//!         ctx.parent_mut().steps += 1; // &mut Robot
//!     }
//! }
//! impl Trigger<Status> for Robot_Walking {
//!     fn handle(&mut self, _: &Status, _: &mut Context<Self>) -> &'static str {
//!         "walking"
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! struct Robot_Silent;
//! impl State for Robot_Silent {
//!     type Parent = Robot;
//! }
//!
//! let mut chart = Statechart::new(Robot::default());
//! assert_eq!(chart.get_debug_name(), "Robot { steps: 0 } -> [Robot_Still, Robot_Silent]");
//! // Neither leaf handles Status, so both branches bubble up to Robot, which runs once.
//! assert_eq!(chart.trigger_all(Status), Ok(vec!["idle"]));
//!
//! chart.trigger(Walk).unwrap();
//! chart.update();
//! assert_eq!(chart.get_debug_name(), "Robot { steps: 1 } -> [Robot_Walking, Robot_Silent]");
//! // Region 0 is handled by Robot_Walking; region 1 still bubbles up to Robot.
//! assert_eq!(chart.trigger_all(Status), Ok(vec!["walking", "idle"]));
//! assert_eq!(chart.trigger(Status), Ok("walking")); // the first output in execution order
//! assert!(chart.is_active::<Robot_Silent>());
//! ```
//!
//! # Lifecycle and ordering
//!
//! * [`Statechart::new`](statechart::Statechart::new) enters the root and then every initial
//!   descendant.
//! * `on_enter` runs parent before child, initial children included, regions in declaration
//!   order. A child the library builds itself (an initial child, or an inactive ancestor of a
//!   transition target) is built right after its parent's `on_enter`, so a constructor given to
//!   `region_with` / `child_with` sees the parent's data as `on_enter` left it.
//! * `on_exit` runs deepest first: a state's descendants exit (regions in declaration order)
//!   before the state itself.
//! * `update` runs parent before children, regions in declaration order.
//!
//! # Transitions
//!
//! A transition targets any state of the same chart, at any depth. Applying it:
//!
//! 1. finds the least common ancestor (LCA): the deepest active state on the target's path
//!    from the root;
//! 2. exits the active subtree of the LCA's region that leads to the target (deepest first);
//! 3. enters every state from there down to the target, parent first. Inactive ancestors of the
//!    target are built with the constructor their parent registered, the target is the instance
//!    passed to `transition`, and every region not on the path gets its initial child.
//!
//! Only that one region of the LCA changes: the LCA itself, its other regions and everything
//! above it are untouched. So a transition into one region of an active parallel state leaves
//! its sibling regions alone, while targeting the parallel state itself exits and re-enters all
//! of its regions. Targeting an active state is an external self-transition: it is exited and
//! entered again. Targeting the root type restarts the whole chart.
//!
//! Transitions are requested from `update`, `on_enter` and trigger handlers (`on_exit` gets an
//! [`ExitContext`](context::ExitContext), which cannot request them). Nothing changes in the
//! middle of a traversal. The requests of one `update` or trigger dispatch form a **batch**:
//!
//! * they are applied in request order, which is traversal order (parent before child for
//!   `update`, handler execution order for triggers);
//! * requests made by `on_enter` hooks while a request is applied go to the end of the batch;
//! * a request is dropped if the state instance that made it has been exited by an earlier
//!   transition in the same batch;
//! * more than 10 000 transitions in one batch panic, which catches `on_enter` hooks that keep
//!   requesting transitions in a cycle.
//!
//! # Introspection
//!
//! * `format!("{chart:?}")` and [`get_debug_name`](statechart::Statechart::get_debug_name)
//!   print the active configuration with each state's own `Debug`: `"A -> A_B"` for a single
//!   chain, `"A -> [A_B -> A_B_X, A_C]"` for parallel regions.
//! * [`is_active::<T>()`](statechart::Statechart::is_active) and
//!   [`state::<T>()`](statechart::Statechart::state) query one state type.
//! * [`get_recursive_active_type_ids`](statechart::Statechart::get_recursive_active_type_ids)
//!   lists the active states below the root, deepest first.
//!
//! # Checks
//!
//! Most mistakes are compile errors: a child listed under the wrong parent, a transition to a
//! state of another chart, registering a trigger the state does not implement, calling
//! `parent()` on the root, or requesting a transition in `on_exit`. A few are panics when the
//! chart is built, in debug and release builds alike: a state type that appears more than once
//! in the hierarchy, a trigger registered twice, or a composite state registered without a
//! constructor. A transition to a state that its parent does not list panics in the hook that
//! requests it, before anything changes.
//!
//! One mistake cannot be detected: implementing `Trigger<E>` but forgetting
//! `def.trigger::<E>()`. That handler never runs, and the trigger is `Err(NotHandled)` unless
//! another active state handles it.
//!
//! # Imports
//!
//! Everything a state or a chart needs is in the [`prelude`]: `use statechart::prelude::*;`.
//! The items themselves live in the modules [`state`], [`definition`], [`context`], [`trigger`]
//! and [`statechart`], and can always be named through those paths. The crate root re-exports
//! nothing else.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(non_camel_case_types)]

pub mod context;
pub mod definition;
mod engine;
pub mod state;
pub mod statechart;
pub mod trigger;

/// Everything needed to write states and run a chart: `use statechart::prelude::*;`.
///
/// Each name here is the same item as the one reached through its module path, e.g.
/// `statechart::prelude::State` is [`statechart::state::State`](crate::state::State).
pub mod prelude {
    pub use crate::context::{Context, ExitContext};
    pub use crate::definition::Definition;
    pub use crate::state::{NoParent, State};
    pub use crate::statechart::Statechart;
    pub use crate::trigger::{Event, NotHandled, Trigger};
}
