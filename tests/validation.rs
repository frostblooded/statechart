//! Mistakes the compiler cannot see, and robustness. The mistakes the compiler *can* see are
//! `compile_fail` doctests in `src/`. Every check here is always on (debug and release), and so
//! is the duplicate-state check, which is an `assert!` (user decision D1; tested in
//! `integration_test.rs`, `test_2`).
#![allow(non_camel_case_types)]

use statechart::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

mod double_trigger {
    use super::*;

    pub struct Foo;
    impl Event for Foo {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Foo>().trigger::<Foo>();
        }
    }
    impl Trigger<Foo> for Root {
        fn handle(&mut self, _: &Foo, _: &mut Context<Self>) {}
    }
}

#[test]
#[should_panic(
    expected = "`validation::double_trigger::Root` registers the trigger `validation::double_trigger::Foo` twice"
)]
fn a_trigger_registered_twice_panics_at_construction() {
    Statechart::new(double_trigger::Root);
}

mod explicit_composite {
    use super::*;

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child_explicit::<Root_B>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
    }

    /// Has a region, so the library may have to build it: `child_explicit` is not allowed.
    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_B_X>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B_X;
    impl State for Root_B_X {
        type Parent = Root_B;
    }
}

#[test]
#[should_panic(
    expected = "State `validation::explicit_composite::Root_B` has regions but was registered with `child_explicit`: a composite state can be entered on the way to one of its descendants, so it needs a constructor; use `child::<validation::explicit_composite::Root_B>()` (requires `Default`) or `child_with(...)`"
)]
fn a_composite_registered_without_a_constructor_panics_at_construction() {
    Statechart::new(explicit_composite::Root);
}

mod cycle {
    use super::*;

    // Loop -> Loop_A (initial) <-> Loop_B: each one's on_enter requests the other.

    #[derive(Debug)]
    pub struct Loop;
    impl State for Loop {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Loop_A>().child::<Loop_B>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_A;
    impl State for Loop_A {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Loop_B);
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_B;
    impl State for Loop_B {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Loop_A);
        }
    }
}

#[test]
#[should_panic(
    expected = "more than 10000 transitions in one batch: do `on_enter` hooks keep requesting transitions in a cycle?"
)]
fn an_on_enter_transition_cycle_panics_instead_of_hanging() {
    Statechart::new(cycle::Loop);
}

mod fragile {
    use super::*;

    // Root { boom } -> Root_A (initial) | Root_B | Root_Ghost (never listed)

    #[derive(Debug)]
    pub struct Root {
        pub boom: bool,
        pub ghost: bool,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            if self.boom {
                ctx.transition(Root_B);
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn update(&mut self, ctx: &mut Context<Self>) {
            let root = ctx.parent_mut();
            if root.ghost {
                root.ghost = false;
                ctx.transition(Root_Ghost);
            }
            let root = ctx.parent_mut();
            if root.boom {
                root.boom = false;
                panic!("boom");
            }
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            panic!("Root_A must not be exited");
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
    }

    /// Names `Root` as its parent, but `Root::define` does not list it.
    #[derive(Debug, Default)]
    pub struct Root_Ghost;
    impl State for Root_Ghost {
        type Parent = Root;
    }
}

#[test]
fn an_unlisted_target_panics_in_the_requesting_hook_before_anything_changes() {
    let mut chart = Statechart::new(fragile::Root {
        boom: false,
        ghost: true,
    });
    let panic = catch_unwind(AssertUnwindSafe(|| chart.update())).unwrap_err();
    let message = panic.downcast_ref::<String>().unwrap();
    assert!(message.starts_with("Transition target `validation::fragile::Root_Ghost`"));
    // Root_A was not exited (its on_exit would panic) and the chart keeps working.
    assert!(chart.is_active::<fragile::Root_A>());
    chart.update();
    assert_eq!(
        chart.get_debug_name(),
        "Root { boom: false, ghost: false } -> Root_A"
    );
}

#[test]
fn a_panicking_hook_leaves_the_chart_usable_and_drops_the_interrupted_requests() {
    let mut chart = Statechart::new(fragile::Root {
        boom: true,
        ghost: false,
    });
    // Root requests Root_B, then Root_A panics before the batch is applied.
    assert!(catch_unwind(AssertUnwindSafe(|| chart.update())).is_err());
    assert!(chart.is_active::<fragile::Root_A>());
    // The interrupted update's request is not applied by the next one.
    chart.update();
    assert_eq!(
        chart.get_debug_name(),
        "Root { boom: false, ghost: false } -> Root_A"
    );
}
