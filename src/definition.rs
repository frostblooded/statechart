//! Declaring a state's regions, children and triggers (the argument of [`State::define`]).

use crate::engine::{Ctor, ErasedState, HandlerFn, RegisterFn, call_handler, register};
use crate::state::State;
use crate::trigger::{Event, Trigger};
use std::any::{TypeId, type_name};
use std::marker::PhantomData;

/// Collects what [`State::define`] declares for the state `S`.
///
/// ```
/// # #![allow(non_camel_case_types)]
/// # use statechart::prelude::*;
/// # struct Reset; impl Event for Reset { type Output = (); }
/// # #[derive(Debug, Default)] struct Player_Idle;
/// # impl State for Player_Idle { type Parent = Player; }
/// # #[derive(Debug, Default)] struct Player_Walking;
/// # impl State for Player_Walking { type Parent = Player; }
/// # #[derive(Debug, Default)] struct Player_Holstered;
/// # impl State for Player_Holstered { type Parent = Player; }
/// # #[derive(Debug)] struct Player_Aiming { target: u32 }
/// # impl State for Player_Aiming { type Parent = Player; }
/// # impl Trigger<Reset> for Player { fn handle(&mut self, _: &Reset, _: &mut Context<Self>) {} }
/// #[derive(Debug)]
/// struct Player;
///
/// impl State for Player {
///     type Parent = NoParent;
///     fn define(def: &mut Definition<Self>) {
///         // Region 0: starts in Player_Idle, may also be in Player_Walking.
///         def.region::<Player_Idle>().child::<Player_Walking>();
///         // Region 1, in parallel: Player_Aiming carries data, so it is only ever entered by
///         // `ctx.transition(Player_Aiming { target })`.
///         def.region::<Player_Holstered>().child_explicit::<Player_Aiming>();
///         def.trigger::<Reset>();
///     }
/// }
/// ```
pub struct Definition<S> {
    regions: Vec<RegionDef>,
    triggers: Vec<(TypeId, HandlerFn)>,
    _state: PhantomData<fn() -> S>,
}

pub(crate) struct ChildDef {
    pub(crate) register: RegisterFn,
    pub(crate) ctor: Option<Ctor>,
}

/// One region: `children[0]` is the initial child.
pub(crate) struct RegionDef {
    pub(crate) children: Vec<ChildDef>,
}

impl<S: State> Definition<S> {
    pub(crate) fn new() -> Self {
        Definition {
            regions: Vec::new(),
            triggers: Vec::new(),
            _state: PhantomData,
        }
    }

    pub(crate) fn into_parts(self) -> (Vec<RegionDef>, Vec<(TypeId, HandlerFn)>) {
        (self.regions, self.triggers)
    }

    /// Declares a region whose initial child is `C`, built with `C::default()` when entered.
    ///
    /// Each call declares one more region; two or more regions run in parallel. A region always
    /// has exactly one initial child, so it cannot be forgotten or duplicated. `C` must name
    /// `Self` as its parent:
    ///
    /// ```compile_fail,E0271
    /// # #![allow(non_camel_case_types)]
    /// # use statechart::prelude::*;
    /// #[derive(Debug)] struct Timer;
    /// #[derive(Debug, Default)] struct Door_Open;
    /// # #[derive(Debug)] struct Door;
    /// # impl State for Door { type Parent = NoParent; }
    /// impl State for Door_Open { type Parent = Door; }
    /// impl State for Timer {
    ///     type Parent = NoParent;
    ///     fn define(def: &mut Definition<Self>) {
    ///         def.region::<Door_Open>(); // error: `Door_Open::Parent` is `Door`, not `Timer`
    ///     }
    /// }
    /// ```
    pub fn region<C>(&mut self) -> Region<'_, S>
    where
        C: State<Parent = S> + Default,
    {
        self.region_with(|_: &S| C::default())
    }

    /// Declares a region whose initial child is built from the parent by `ctor`, e.g.
    /// `def.region_with(|timer: &Timer| Timer_Active { remaining: timer.duration })`.
    ///
    /// `ctor` runs each time the child is entered without an instance, right after the
    /// parent's `on_enter`, so it sees the parent's data as `on_enter` left it.
    pub fn region_with<C>(&mut self, ctor: fn(&S) -> C) -> Region<'_, S>
    where
        C: State<Parent = S>,
    {
        self.regions.push(RegionDef {
            children: Vec::new(),
        });
        let region = self.regions.last_mut().expect("just pushed");
        Region {
            def: region,
            _state: PhantomData,
        }
        .push::<C>(Some(erase_ctor(ctor)))
    }

    /// Declares that `S` handles the trigger `E`, with its `impl Trigger<E>`. Returns `self`,
    /// so several triggers chain: `def.trigger::<Pause>().trigger::<Reset>();`.
    ///
    /// **This call is required.** Implementing [`Trigger<E>`] alone does nothing: without the
    /// registration the handler is never called, the trigger bubbles past `S` to its ancestors,
    /// and if none of them handles it the chart returns `Err(NotHandled)`. The library cannot
    /// detect a forgotten registration, so register each trigger next to the regions in
    /// [`define`](State::define).
    ///
    /// The opposite mistake is a compile error: registering a trigger that `S` does not
    /// implement does not compile.
    ///
    /// ```compile_fail,E0277
    /// # use statechart::prelude::*;
    /// struct Foo;
    /// impl Event for Foo { type Output = (); }
    /// #[derive(Debug)] struct A;
    /// impl State for A {
    ///     type Parent = NoParent;
    ///     fn define(def: &mut Definition<Self>) {
    ///         def.trigger::<Foo>(); // error: `A` does not handle trigger `Foo`
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// When the chart is built, if `S` registers the same trigger twice.
    #[track_caller]
    pub fn trigger<E: Event>(&mut self) -> &mut Self
    where
        S: Trigger<E>,
    {
        let event = TypeId::of::<E>();
        assert!(
            self.triggers.iter().all(|(e, _)| *e != event),
            "`{}` registers the trigger `{}` twice",
            type_name::<S>(),
            type_name::<E>()
        );
        self.triggers.push((event, call_handler::<S, E>));
        self
    }
}

/// Builder for one region of `S`, returned by [`Definition::region`] and
/// [`Definition::region_with`]. Adds the region's other (non-initial) children.
pub struct Region<'d, S> {
    def: &'d mut RegionDef,
    _state: PhantomData<fn() -> S>,
}

impl<S: State> Region<'_, S> {
    fn push<C: State<Parent = S>>(self, ctor: Option<Ctor>) -> Self {
        self.def.children.push(ChildDef {
            register: register::<C>,
            ctor,
        });
        self
    }

    /// Adds the child `C` to this region. When the library has to build `C` itself (because
    /// a transition targets one of `C`'s descendants while `C` is inactive) it uses
    /// `C::default()`.
    pub fn child<C>(self) -> Self
    where
        C: State<Parent = S> + Default,
    {
        self.child_with(|_: &S| C::default())
    }

    /// Adds the child `C`, built from the parent by `ctor` when the library has to build it.
    pub fn child_with<C>(self, ctor: fn(&S) -> C) -> Self
    where
        C: State<Parent = S>,
    {
        self.push::<C>(Some(erase_ctor(ctor)))
    }

    /// Adds the child `C` without a constructor: it is only ever entered by a transition that
    /// targets it with an instance (`ctx.transition(C { .. })`). Use this for leaves whose
    /// data only makes sense when given by the transition.
    ///
    /// # Panics
    ///
    /// When the chart is built, if `C` has regions: a composite state can be entered on the way
    /// to one of its descendants, so the library must be able to build it.
    pub fn child_explicit<C>(self) -> Self
    where
        C: State<Parent = S>,
    {
        self.push::<C>(None)
    }
}

fn erase_ctor<P: State, C: State>(ctor: fn(&P) -> C) -> Ctor {
    Box::new(move |parent: &dyn ErasedState| {
        let parent = parent
            .as_any()
            .downcast_ref::<P>()
            .expect("a child is always built from its own parent");
        Box::new(ctor(parent)) as Box<dyn ErasedState>
    })
}
