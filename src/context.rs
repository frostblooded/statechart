//! The contexts handed to hooks: typed parent access, ancestor lookup, active-state queries and
//! (except in `on_exit`) transitions.

use crate::engine::{Ancestors, ErasedState, Registry, Request, Runtime};
use crate::state::{InChart, RootOf, State};
use std::any::{TypeId, type_name};
use std::marker::PhantomData;

/// What `update`, `on_enter` and trigger handlers of state `S` can see and do.
///
/// * [`parent`](Self::parent) / [`parent_mut`](Self::parent_mut): the direct parent, statically
///   typed, never an `Option` (the root does not have these methods);
/// * [`ancestor`](Self::ancestor) / [`ancestor_mut`](Self::ancestor_mut): any active ancestor,
///   looked up by type;
/// * [`is_active`](Self::is_active): whether a state of the same chart is active;
/// * [`transition`](Self::transition): request a transition, applied after the current
///   update / trigger dispatch / transition finishes.
///
/// Ancestors are reachable through `&self` / `&mut self` only, so the borrow checker
/// guarantees that a reference to ancestor data does not outlive the hook call and is not held
/// across a [`transition`](Self::transition) request.
///
/// ```
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// # #[derive(Debug)] struct Timer_Elapsed;
/// # impl State for Timer_Elapsed { type Parent = Timer; }
/// #[derive(Debug)]
/// struct Timer_Active {
///     remaining: u32,
/// }
/// # #[derive(Debug)] struct Timer;
/// # impl State for Timer { type Parent = NoParent; fn define(d: &mut Definition<Self>) { d.region_with(|_| Timer_Active { remaining: 3 }).child_explicit::<Timer_Elapsed>(); } }
/// # impl State for Timer_Active { type Parent = Timer; fn define(d: &mut Definition<Self>) { d.region::<Timer_Active_Running>(); } }
///
/// #[derive(Debug, Default)]
/// struct Timer_Active_Running;
///
/// impl State for Timer_Active_Running {
///     type Parent = Timer_Active;
///     fn update(&mut self, ctx: &mut Context<Self>) {
///         let active = ctx.parent_mut(); // &mut Timer_Active
///         active.remaining -= 1;
///         if active.remaining == 0 {
///             ctx.transition(Timer_Elapsed); // a sibling of the parent
///         }
///     }
/// }
/// ```
///
/// The root has no parent, so calling `parent()` there does not compile:
///
/// ```compile_fail,E0277
/// # use statechart::prelude::*;
/// #[derive(Debug)]
/// struct Timer;
/// impl State for Timer {
///     type Parent = NoParent;
///     fn update(&mut self, ctx: &mut Context<Self>) {
///         ctx.parent(); // error: `NoParent` is not a statechart state
///     }
/// }
/// ```
pub struct Context<'a, S> {
    hook: Hook<'a>,
    _state: PhantomData<fn() -> S>,
}

/// What `on_exit` of state `S` can see: the same ancestor access as [`Context`], but no
/// `transition`. A state that is being exited cannot request a transition (it would always be
/// dropped), so the method does not exist:
///
/// ```compile_fail,E0599
/// # use statechart::prelude::*;
/// #[derive(Debug)]
/// struct Timer;
/// impl State for Timer {
///     type Parent = NoParent;
///     fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
///         ctx.transition(Timer); // error: no method named `transition`
///     }
/// }
/// ```
pub struct ExitContext<'a, S> {
    hook: Hook<'a>,
    _state: PhantomData<fn() -> S>,
}

/// The untyped part of both contexts: the ancestor frames, the registry and the runtime.
pub(crate) struct Hook<'a> {
    pub(crate) anc: &'a mut dyn Ancestors,
    pub(crate) reg: &'a Registry,
    pub(crate) rt: &'a mut Runtime,
    /// Instance id of the state whose hook runs.
    pub(crate) id: u64,
}

/// Methods shared by [`Context`] and [`ExitContext`].
macro_rules! ancestor_access {
    () => {
        pub(crate) fn new(hook: Hook<'a>) -> Self {
            Self {
                hook,
                _state: PhantomData,
            }
        }

        /// The direct parent's data. Statically typed; the root does not have this method.
        pub fn parent(&self) -> &S::Parent
        where
            S::Parent: State,
        {
            self.hook
                .anc
                .parent()
                .and_then(|p| p.as_any().downcast_ref())
                .expect("the parent frame always holds `S::Parent`")
        }

        /// The direct parent's data, mutably. Statically typed; the root does not have this
        /// method.
        pub fn parent_mut(&mut self) -> &mut S::Parent
        where
            S::Parent: State,
        {
            self.hook
                .anc
                .parent_mut()
                .and_then(|p| p.as_any_mut().downcast_mut())
                .expect("the parent frame always holds `S::Parent`")
        }

        /// The active ancestor of type `A`, at any depth, or `None` if `A` is not an ancestor of
        /// `S`. States in other regions (siblings, cousins) are not ancestors, and neither is
        /// `S` itself.
        pub fn ancestor<A: State>(&self) -> Option<&A> {
            self.hook
                .anc
                .find(TypeId::of::<A>())
                .and_then(|a| a.as_any().downcast_ref())
        }

        /// Mutable version of [`ancestor`](Self::ancestor).
        pub fn ancestor_mut<A: State>(&mut self) -> Option<&mut A> {
            self.hook
                .anc
                .find_mut(TypeId::of::<A>())
                .and_then(|a| a.as_any_mut().downcast_mut())
        }

        /// Whether a state of type `T` is active right now. `T` can be anywhere in the chart,
        /// including another region (the classic "in state" guard). `S` itself is always
        /// active while its hooks run.
        ///
        /// During a transition, states count as active from just before their `on_enter` until
        /// just after their `on_exit`.
        pub fn is_active<T: InChart<RootOf<S>>>(&self) -> bool {
            self.hook.rt.is_live(TypeId::of::<T>())
        }
    };
}

impl<'a, S: State> Context<'a, S> {
    ancestor_access!();

    /// Requests a transition to `target`, which becomes the active state of its region. Its
    /// inactive ancestors are built and entered first (each with the constructor its parent
    /// registered), and its regions get their initial children.
    ///
    /// Applied after the current `update` / trigger dispatch finishes (or, for requests made in
    /// `on_enter`, after the current transition), in request order. A request is dropped if
    /// the state that made it has been exited by then. Targeting the active state of the same
    /// type is an external self-transition (exit, then enter `target`).
    ///
    /// The transition exits and re-enters as little as possible. The deepest active state on
    /// the target's path from the root (the least common ancestor) stays, and only its region
    /// that leads to `target` is exited and entered again. Its other regions and everything
    /// above it are untouched, so a transition into one region of a parallel state leaves the
    /// sibling regions as they are. To reset every region, target the parallel state itself.
    ///
    /// Only states of the same chart are accepted:
    ///
    /// ```compile_fail,E0277
    /// # #![allow(non_camel_case_types)]
    /// # use statechart::prelude::*;
    /// #[derive(Debug)] struct Timer;
    /// #[derive(Debug)] struct Door;
    /// #[derive(Debug)] struct Door_Open;
    /// impl State for Door { type Parent = NoParent; }
    /// impl State for Door_Open { type Parent = Door; }
    /// impl State for Timer {
    ///     type Parent = NoParent;
    ///     fn update(&mut self, ctx: &mut Context<Self>) {
    ///         ctx.transition(Door_Open); // error: `Door_Open` belongs to the `Door` chart
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// If `T`'s parent does not list `T` in its [`define`](State::define) (the compiler checks
    /// `type Parent`, but cannot check that the parent lists the child). The panic happens here,
    /// in the hook that made the request, before anything changes.
    #[track_caller]
    pub fn transition<T: InChart<RootOf<S>>>(&mut self, target: T) {
        let Some(target_idx) = self.hook.reg.index_of(TypeId::of::<T>()) else {
            panic!(
                "Transition target `{}` is not part of this statechart: its parent `{}` does not \
                 list it in `define`",
                type_name::<T>(),
                type_name::<T::Parent>()
            );
        };
        self.hook.rt.requests.push(Request {
            requester_ty: TypeId::of::<S>(),
            requester: self.hook.id,
            target_idx,
            target: Box::new(target) as Box<dyn ErasedState>,
        });
    }
}

impl<'a, S: State> ExitContext<'a, S> {
    ancestor_access!();
}
