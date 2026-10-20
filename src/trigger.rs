//! Triggers: typed events that bubble from the active leaves towards the root.

use crate::context::Context;
use crate::state::State;
use std::fmt;

/// A trigger type (the event a statechart is triggered with) and what handling it returns.
///
/// ```
/// use statechart::trigger::Event;
///
/// /// Asks the active state how many seconds are left.
/// struct GetRemaining;
///
/// impl Event for GetRemaining {
///     type Output = u32;
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a trigger",
    label = "`{Self}` does not implement `Event`",
    note = "add `impl statechart::trigger::Event for {Self} {{ type Output = ...; }}`"
)]
pub trait Event: 'static {
    /// The value a handler returns. [`Statechart::trigger`](crate::statechart::Statechart::trigger)
    /// gives it back as `Ok(output)`. Use `()` for plain commands.
    type Output: 'static;
}

/// A state's handler for the trigger `E`.
///
/// Handling a trigger takes two steps: implement this trait, **and** register the trigger in
/// [`State::define`] with [`def.trigger::<E>()`](crate::definition::Definition::trigger):
///
/// ```
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// struct Pause;
/// impl Event for Pause {
///     type Output = ();
/// }
///
/// #[derive(Debug)]
/// struct Timer {
///     paused: bool,
/// }
///
/// impl State for Timer {
///     type Parent = NoParent;
///     fn define(def: &mut Definition<Self>) {
///         def.trigger::<Pause>(); // step 2: without this line, `handle` is never called
///     }
/// }
///
/// impl Trigger<Pause> for Timer {
///     // step 1
///     fn handle(&mut self, _: &Pause, _: &mut Context<Self>) {
///         self.paused = true;
///     }
/// }
///
/// let mut chart = Statechart::new(Timer { paused: false });
/// assert_eq!(chart.trigger(Pause), Ok(()));
/// assert!(chart.root().paused);
/// ```
///
/// # Forgetting the registration
///
/// The registration is what the library dispatches on; the trait impl alone is invisible to it.
/// **A forgotten `def.trigger::<E>()` is not detected**, neither at compile time nor at run
/// time: the handler silently never runs, the trigger bubbles on to the state's ancestors, and
/// if none of them handles it [`Statechart::trigger`](crate::statechart::Statechart::trigger)
/// returns `Err(NotHandled)`. (The reverse, registering a trigger without implementing this
/// trait, does not compile.)
///
/// # Dispatch
///
/// A trigger bubbles from each active leaf towards the root. On each branch the deepest active
/// state that registered `E` handles it, and only that state runs. With parallel regions every
/// branch bubbles on its own; an ancestor shared by several branches runs at most once, and only
/// if at least one branch below it was not handled.
///
/// The handler gets the trigger by reference because, with parallel regions, several states may
/// handle the same trigger. The return type is fixed by the trigger:
///
/// ```compile_fail,E0053
/// # use statechart::prelude::*;
/// struct GetRemaining;
/// impl Event for GetRemaining { type Output = u32; }
/// #[derive(Debug)] struct Timer;
/// impl State for Timer { type Parent = NoParent; }
/// impl Trigger<GetRemaining> for Timer {
///     fn handle(&mut self, _: &GetRemaining, _: &mut Context<Self>) -> &'static str { "3" } // error
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not handle trigger `{E}`",
    label = "`{Self}` has no `Trigger<{E}>` impl",
    note = "add `impl Trigger<{E}> for {Self} {{ fn handle(&mut self, trigger: &{E}, ctx: &mut Context<Self>) -> <{E} as Event>::Output {{ ... }} }}`"
)]
pub trait Trigger<E: Event>: State {
    /// Handles the trigger. Runs only if no active descendant on this branch handled it.
    ///
    /// The context can read and change ancestors and request transitions, which are applied
    /// after the whole trigger dispatch finishes.
    fn handle(&mut self, trigger: &E, ctx: &mut Context<'_, Self>) -> E::Output;
}

/// Returned when no active state handles a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotHandled;

impl fmt::Display for NotHandled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no active state handles this trigger")
    }
}

impl std::error::Error for NotHandled {}
