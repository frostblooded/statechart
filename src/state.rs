//! The [`State`] trait and the type-level description of the hierarchy.
//!
//! Every state names its parent through the associated type [`State::Parent`]. That single
//! line gives the library *static* knowledge of the hierarchy:
//!
//! * [`Context::parent`] is typed (`&Self::Parent`, never an `Option`), and the root cannot call
//!   it at all;
//! * a parent can only list children whose `Parent` is itself (checked by the compiler);
//! * the chain `T -> T::Parent -> ... -> root` of any transition target is known, so a
//!   transition can enter ancestors that are not active yet;
//! * [`InChart`] turns "is this state part of my chart?" into a compile-time question.

use crate::context::{Context, ExitContext};
use crate::definition::Definition;
use std::any::Any;
use std::fmt::Debug;

/// A state of a statechart. The struct's fields are the state's data.
///
/// Every method has a default, so the smallest state is one line plus its parent:
///
/// ```
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// # #[derive(Debug)] struct Timer;
/// # impl State for Timer { type Parent = NoParent; fn define(d: &mut Definition<Self>) { d.region::<Timer_Elapsed>(); } }
/// #[derive(Debug, Default)]
/// struct Timer_Elapsed;
///
/// impl State for Timer_Elapsed {
///     type Parent = Timer;
/// }
/// ```
///
/// `Debug` is used by [`Statechart::get_debug_name`](crate::statechart::Statechart::get_debug_name);
/// `Any` (which implies `'static`) lets the library store states of different types side by
/// side.
///
/// Hooks take a context: write `ctx: &mut Context<Self>` (or `Context<'_, Self>`) for
/// `on_enter` and `update`, and `ctx: &mut ExitContext<Self>` for `on_exit`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a statechart state",
    label = "`{Self}` does not implement `State`",
    note = "a state is a type that derives `Debug` and implements `statechart::state::State` (`impl State for {Self} {{ type Parent = ...; }}`)"
)]
pub trait State: Any + Debug + Sized {
    /// The state this one is a child of, or [`NoParent`] for the root.
    ///
    /// Because every state type appears at most once in a hierarchy, a state has exactly one
    /// parent type. The parent must also list this state in its [`define`](State::define).
    type Parent: ParentSlot;

    /// Declares this state's regions (and the children of each region) and the triggers it
    /// handles.
    ///
    /// Called once per [`Statechart`](crate::statechart::Statechart), when it is built, without
    /// an instance: the hierarchy is a property of the types. A leaf keeps the default (no
    /// regions); a plain composite declares one region; two or more regions run in parallel.
    /// See [`Definition`] for an example.
    ///
    /// **Every handled trigger must be registered here** with
    /// [`def.trigger::<E>()`](Definition::trigger). An `impl Trigger<E>` that is not registered
    /// is never called, and nothing warns about it: the trigger then falls back to an ancestor,
    /// or is `Err(NotHandled)`.
    fn define(def: &mut Definition<Self>) {
        let _ = def;
    }

    /// Runs when the state becomes active: after its parent's `on_enter`, before its
    /// children's. May request transitions; they are applied after the transition (or the
    /// construction) that is entering this state finishes.
    fn on_enter(&mut self, ctx: &mut Context<'_, Self>) {
        let _ = ctx;
    }

    /// Runs when the state stops being active, after all of its descendants' `on_exit`.
    ///
    /// Takes an [`ExitContext`], which has ancestor access but no `transition`: a state that
    /// is being exited cannot request transitions (they would always be dropped). Writing it
    /// with a [`Context`] does not compile:
    ///
    /// ```compile_fail,E0053
    /// # use statechart::prelude::*;
    /// #[derive(Debug)] struct Timer;
    /// impl State for Timer {
    ///     type Parent = NoParent;
    ///     fn on_exit(&mut self, ctx: &mut Context<Self>) {} // error: expected `ExitContext`
    /// }
    /// ```
    fn on_exit(&mut self, ctx: &mut ExitContext<'_, Self>) {
        let _ = ctx;
    }

    /// Runs on every [`Statechart::update`](crate::statechart::Statechart::update): parent before
    /// children, regions in declaration order.
    fn update(&mut self, ctx: &mut Context<'_, Self>) {
        let _ = ctx;
    }
}

/// The parent of the root state (`type Parent = NoParent;`). It has no values; it only exists
/// as a type.
#[derive(Debug)]
pub enum NoParent {}

/// Crate-private supertraits and helpers of the public traits. Code outside the crate can
/// neither name nor implement them, and cannot run their methods, not even through a
/// `ParentSlot` bound:
///
/// ```compile_fail,E0576
/// # use statechart::prelude::*;
/// use statechart::state::ParentSlot;
/// #[derive(Debug)]
/// struct Door;
/// impl State for Door {
///     type Parent = NoParent;
/// }
/// <Door as ParentSlot>::unlisted_link(&|_: std::any::TypeId| false); // error: not in `ParentSlot`
/// ```
///
/// ```compile_fail,E0308
/// use statechart::state::ParentSlot;
/// fn leak<T: ParentSlot>() {
///     T::unlisted_link(&|_: std::any::TypeId| false); // error: only the crate can build the argument
/// }
/// ```
pub(crate) mod sealed {
    use super::{NoParent, State};
    use std::any::{TypeId, type_name};

    /// Which state types the chart registered. Only the crate can build one, so code outside
    /// it cannot run [`Sealed::unlisted_link`].
    #[derive(Clone, Copy)]
    pub struct Registered<'a>(pub(crate) &'a dyn Fn(TypeId) -> bool);

    /// The supertrait of [`ParentSlot`](super::ParentSlot), implemented for `NoParent` and
    /// every state.
    pub trait Sealed {
        /// For a panic message: if `Self` is a state that is not registered, the highest
        /// unregistered state on the way up from `Self` and its (registered) parent.
        fn unlisted_link(registered: Registered<'_>) -> Option<(&'static str, &'static str)>;
    }

    impl Sealed for NoParent {
        fn unlisted_link(_: Registered<'_>) -> Option<(&'static str, &'static str)> {
            None
        }
    }

    impl<P: State> Sealed for P {
        fn unlisted_link(registered: Registered<'_>) -> Option<(&'static str, &'static str)> {
            if (registered.0)(TypeId::of::<P>()) {
                return None;
            }
            Some(
                <P::Parent as Sealed>::unlisted_link(registered)
                    .unwrap_or((type_name::<P>(), type_name::<P::Parent>())),
            )
        }
    }

    /// A parent with data, that is, any state: the bound of `parent()` / `parent_mut()`. Its
    /// own trait (instead of `State`) so that the root's missing `parent()` gets its own
    /// message.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a statechart state",
        label = "the root state has no parent",
        note = "a root state has no parent, so its context has no `parent()` or `parent_mut()`"
    )]
    pub trait ParentState: super::State {}
    impl<P: super::State> ParentState for P {}

    /// Type equality as a trait bound, so that a mismatch is reported with a readable message.
    #[diagnostic::on_unimplemented(
        message = "this state belongs to the statechart rooted at `{Self}`, not the one rooted at `{R}`",
        label = "belongs to the statechart rooted at `{Self}`",
        note = "following `type Parent` up from a state must end at the root of the chart it is used with"
    )]
    pub trait SameRoot<R> {}
    impl<R> SameRoot<R> for R {}
}

/// What may appear as [`State::Parent`]: another [`State`] or [`NoParent`]. Sealed.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be a parent",
    label = "`{Self}` is neither a statechart state nor `NoParent`",
    note = "`type Parent` names another state of the same chart, or `NoParent` for the root"
)]
pub trait ParentSlot: sealed::Sealed + 'static {
    /// The root of the chart that a state with this parent belongs to.
    #[doc(hidden)]
    type RootOf<Child: State>: State;
}

impl ParentSlot for NoParent {
    type RootOf<Child: State> = Child;
}

impl<P: State> ParentSlot for P {
    type RootOf<Child: State> = <P::Parent as ParentSlot>::RootOf<P>;
}

/// The root state type of the chart `S` belongs to, computed from the `Parent` chain.
pub type RootOf<S> = <<S as State>::Parent as ParentSlot>::RootOf<S>;

/// Implemented automatically by every state of the chart whose root is `R`.
///
/// Used to reject, at compile time, transitions to states of another chart and queries such as
/// [`Statechart::is_active`](crate::statechart::Statechart::is_active) for foreign states.
///
/// A state of another chart is reported as such ("this state belongs to the statechart rooted
/// at ..."). Anything else, such as a boxed state (`ctx.transition(Box::new(..))`) or a type
/// that is not a state at all, gets this trait's own message:
///
/// ```compile_fail,E0277
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// # #[derive(Debug)] struct Door;
/// # impl State for Door { type Parent = NoParent; fn define(d: &mut Definition<Self>) { d.region::<Door_Closed>().child::<Door_Open>(); } }
/// # #[derive(Debug, Default)] struct Door_Open;
/// # impl State for Door_Open { type Parent = Door; }
/// #[derive(Debug, Default)]
/// struct Door_Closed;
/// impl State for Door_Closed {
///     type Parent = Door;
///     fn update(&mut self, ctx: &mut Context<Self>) {
///         ctx.transition(Box::new(Door_Open)); // error: pass the state by value
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a state of the statechart rooted at `{R}`",
    label = "not a state type of this chart",
    note = "the chart only accepts its own state types: types that implement `State` and whose `type Parent` chain ends at `{R}`; pass a state by value (`ctx.transition(Target {{ .. }})`), not boxed or by reference"
)]
pub trait InChart<R>: State {}

impl<S: State, R> InChart<R> for S where RootOf<S>: sealed::SameRoot<R> {}
