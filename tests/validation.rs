//! Mistakes the compiler cannot see, and robustness. The mistakes the compiler *can* see are
//! `compile_fail` doctests in `src/`. Every check here is always on (debug and release), and so
//! is the duplicate-state check, which is an `assert!` (user decision D1; tested in
//! `integration_test.rs`, `test_2`, and in `panic_locations.rs`). CI also runs these tests in a
//! release build. Robustness: a panicking hook, whether during a traversal or while a
//! transition is applied, leaves the chart consistent and usable.
#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use common::{take_log, take_log_without_updates};
use statechart::prelude::*;
use std::any::TypeId;
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

/// The message of a panic caught with `catch_unwind`.
fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    match panic.downcast::<String>() {
        Ok(message) => *message,
        Err(panic) => panic.downcast_ref::<&str>().unwrap().to_string(),
    }
}

#[test]
fn the_cycle_guard_names_the_states_that_keep_requesting_transitions() {
    let panic = catch_unwind(|| Statechart::new(cycle::Loop)).unwrap_err();
    let message = panic_message(panic);
    assert!(
        message.ends_with(
            "(the last request: `validation::cycle::Loop_A` requested `validation::cycle::Loop_B`)"
        ),
        "{message}"
    );
}

mod unlisted_ancestor {
    use super::*;

    // Root lists only Root_A. Root_G names Root as its parent and lists Root_G_X, which lists
    // Root_G_X_Y: both are correct, but unreachable, because Root forgot Root_G.

    #[derive(Debug)]
    pub enum Target {
        Child,
        GrandChild,
    }

    #[derive(Debug)]
    pub struct Root {
        pub target: Target,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn update(&mut self, ctx: &mut Context<Self>) {
            match ctx.parent().target {
                Target::Child => ctx.transition(Root_G_X),
                Target::GrandChild => ctx.transition(Root_G_X_Y),
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_G;
    impl State for Root_G {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_G_X>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_G_X;
    impl State for Root_G_X {
        type Parent = Root_G;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_G_X_Y>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_G_X_Y;
    impl State for Root_G_X_Y {
        type Parent = Root_G_X;
    }
}

#[test]
fn an_unlisted_ancestor_is_named_instead_of_the_targets_own_parent() {
    use unlisted_ancestor::*;
    for (target, name) in [
        (Target::Child, "Root_G_X"),
        (Target::GrandChild, "Root_G_X_Y"),
    ] {
        let mut chart = Statechart::new(Root { target });
        let panic = catch_unwind(AssertUnwindSafe(|| chart.update())).unwrap_err();
        assert_eq!(
            panic_message(panic),
            format!(
                "Transition target `validation::unlisted_ancestor::{name}` is not part of this \
                 statechart: its ancestor `validation::unlisted_ancestor::Root_G` is not listed \
                 by its parent `validation::unlisted_ancestor::Root` in `define`"
            )
        );
    }
}

/// A trigger handler that requests a transition and then panics.
mod leaky {
    use super::*;

    pub struct Boom;
    impl Event for Boom {
        type Output = ();
    }

    pub struct Noop;
    impl Event for Noop {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Boom>().trigger::<Noop>();
        }
    }
    impl Trigger<Boom> for Root_A {
        fn handle(&mut self, _: &Boom, ctx: &mut Context<Self>) {
            ctx.transition(Root_B);
            panic!("boom");
        }
    }
    impl Trigger<Noop> for Root_A {
        fn handle(&mut self, _: &Noop, _: &mut Context<Self>) {}
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
    }
}

#[test]
fn a_panicking_trigger_handler_does_not_leak_its_request_into_the_next_trigger() {
    use leaky::*;
    let mut chart = Statechart::new(Root);
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger(Boom))).is_err());
    assert_eq!(chart.trigger(Noop), Ok(()));
    assert_eq!(chart.get_debug_name(), "Root -> Root_A");
}

/// A hook (or a constructor) that panics while a transition is applied.
mod mid_transition {
    use super::*;

    // Root { panic_in }
    // ├── region 0: Root_A (initial), Root_B, Root_C { boom }
    // │   Root_A
    // │   └── region: Root_A_X (initial)
    // │   Root_C
    // │   └── region: Root_C_X (initial; its constructor panics if `boom`)
    // └── region 1: Root_P (initial)
    //
    // Every hook logs itself, then panics if the root's `panic_in` names it ("enter Root_B").
    // The states of region 0 handle `GoTo`; the root handles `Reset` (-> Root_A) and `Restart`
    // (-> a new root whose `on_enter` panics).

    pub enum To {
        A,
        B,
        CBoom,
    }

    pub struct GoTo(pub To);
    impl Event for GoTo {
        type Output = ();
    }

    pub struct Reset;
    impl Event for Reset {
        type Output = ();
    }

    pub struct Restart;
    impl Event for Restart {
        type Output = ();
    }

    fn run_hook(panic_in: &str, hook: &str) {
        let (what, who) = hook.split_once(' ').unwrap();
        common::log(what, who);
        if panic_in == hook {
            panic!("{hook} panicked");
        }
    }

    macro_rules! hooks {
        ($name:literal) => {
            fn on_enter(&mut self, ctx: &mut Context<Self>) {
                run_hook(
                    ctx.ancestor::<Root>().unwrap().panic_in,
                    concat!("enter ", $name),
                );
            }
            fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
                run_hook(
                    ctx.ancestor::<Root>().unwrap().panic_in,
                    concat!("exit ", $name),
                );
            }
        };
    }

    macro_rules! go_to {
        ($state:ty) => {
            impl Trigger<GoTo> for $state {
                fn handle(&mut self, go: &GoTo, ctx: &mut Context<Self>) {
                    match go.0 {
                        To::A => ctx.transition(Root_A),
                        To::B => ctx.transition(Root_B),
                        To::CBoom => ctx.transition(Root_C { boom: true }),
                    }
                }
            }
        };
    }

    pub struct Root {
        pub panic_in: &'static str,
    }
    impl std::fmt::Debug for Root {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Root")
        }
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>().child::<Root_C>();
            def.region::<Root_P>();
            def.trigger::<Reset>().trigger::<Restart>();
        }
        fn on_enter(&mut self, _: &mut Context<Self>) {
            run_hook(self.panic_in, "enter Root");
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            run_hook(self.panic_in, "exit Root");
        }
    }
    impl Trigger<Reset> for Root {
        fn handle(&mut self, _: &Reset, ctx: &mut Context<Self>) {
            ctx.transition(Root_A);
        }
    }
    impl Trigger<Restart> for Root {
        fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
            ctx.transition(Root {
                panic_in: "enter Root",
            });
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A_X>();
            def.trigger::<GoTo>();
        }
        hooks!("Root_A");
    }
    go_to!(Root_A);

    #[derive(Debug, Default)]
    pub struct Root_A_X;
    impl State for Root_A_X {
        type Parent = Root_A;
        hooks!("Root_A_X");
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<GoTo>();
        }
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            let panic_in = ctx.parent().panic_in;
            run_hook(panic_in, "enter Root_B");
            if panic_in == "ghost" {
                ctx.transition(Root_Ghost);
            }
        }
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            run_hook(ctx.parent().panic_in, "exit Root_B");
        }
    }
    go_to!(Root_B);

    #[derive(Debug, Default)]
    pub struct Root_C {
        pub boom: bool,
    }
    impl State for Root_C {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region_with(|c: &Root_C| {
                assert!(!c.boom, "the constructor of Root_C_X panicked");
                Root_C_X
            });
            def.trigger::<GoTo>();
        }
        hooks!("Root_C");
    }
    go_to!(Root_C);

    #[derive(Debug)]
    pub struct Root_C_X;
    impl State for Root_C_X {
        type Parent = Root_C;
        hooks!("Root_C_X");
    }

    #[derive(Debug, Default)]
    pub struct Root_P;
    impl State for Root_P {
        type Parent = Root;
        hooks!("Root_P");
    }

    /// Names `Root` as its parent, but `Root::define` does not list it.
    #[derive(Debug, Default)]
    pub struct Root_Ghost;
    impl State for Root_Ghost {
        type Parent = Root;
    }

    pub fn new_chart(panic_in: &'static str) -> Statechart<Root> {
        let chart = Statechart::new(Root { panic_in });
        take_log();
        chart
    }

    /// Runs `f`, which must panic.
    pub fn expect_panic(f: impl FnOnce()) -> String {
        panic_message(catch_unwind(AssertUnwindSafe(f)).unwrap_err())
    }
}

#[test]
fn a_panic_in_on_enter_during_a_transition_leaves_the_chart_consistent() {
    use mid_transition::*;
    let mut chart = new_chart("enter Root_B");
    expect_panic(|| chart.trigger(GoTo(To::B)).unwrap());
    assert_eq!(take_log(), ["exit Root_A_X", "exit Root_A", "enter Root_B"]);
    // Root_A is gone (it was exited); Root_B was entered, even though its `on_enter` did not
    // finish. The tree, `is_active` and `state` agree.
    assert_eq!(chart.get_debug_name(), "Root -> [Root_B, Root_P]");
    assert!(!chart.is_active::<Root_A>());
    assert!(chart.state::<Root_A>().is_none());
    assert!(!chart.is_active::<Root_A_X>());
    assert!(chart.is_active::<Root_B>());
    assert!(chart.state::<Root_B>().is_some());
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        [TypeId::of::<Root_B>(), TypeId::of::<Root_P>()]
    );

    // Root_B's requests work, and Root_A is not exited a second time.
    chart.root_mut().panic_in = "";
    chart.trigger(GoTo(To::A)).unwrap();
    assert_eq!(
        take_log(),
        ["exit Root_B", "enter Root_A", "enter Root_A_X"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root -> [Root_A -> Root_A_X, Root_P]"
    );
}

#[test]
fn a_panic_in_on_exit_during_a_transition_leaves_the_region_empty() {
    use mid_transition::*;
    let mut chart = new_chart("exit Root_A_X");
    expect_panic(|| chart.trigger(GoTo(To::B)).unwrap());
    assert_eq!(take_log(), ["exit Root_A_X"]);
    // The subtree being exited is dropped (Root_A without its `on_exit`), and Root_B was never
    // entered: region 0 has no active state.
    assert_eq!(chart.get_debug_name(), "Root -> [(empty), Root_P]");
    assert!(!chart.is_active::<Root_A>());
    assert!(!chart.is_active::<Root_A_X>());
    assert!(!chart.is_active::<Root_B>());
    assert!(chart.state::<Root_A>().is_none());
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        [TypeId::of::<Root_P>()]
    );

    // The empty region handles nothing (its branch bubbles up to Root, which does not handle
    // GoTo), and updates and triggers keep working.
    assert_eq!(chart.trigger(GoTo(To::B)), Err(NotHandled));
    chart.update();
    chart.root_mut().panic_in = "";
    chart.trigger(Reset).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["enter Root_A", "enter Root_A_X"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root -> [Root_A -> Root_A_X, Root_P]"
    );
}

#[test]
fn a_panic_in_a_constructor_during_a_transition_leaves_the_new_states_region_empty() {
    use mid_transition::*;
    let mut chart = new_chart("");
    let message = expect_panic(|| chart.trigger(GoTo(To::CBoom)).unwrap());
    assert_eq!(message, "the constructor of Root_C_X panicked");
    assert_eq!(take_log(), ["exit Root_A_X", "exit Root_A", "enter Root_C"]);
    assert_eq!(
        chart.get_debug_name(),
        "Root -> [Root_C { boom: true } -> (empty), Root_P]"
    );
    assert!(chart.is_active::<Root_C>());
    assert!(!chart.is_active::<Root_C_X>());
    assert!(chart.state::<Root_C_X>().is_none());

    // Root_C handles GoTo for its empty region and can leave.
    chart.trigger(GoTo(To::A)).unwrap();
    assert_eq!(
        take_log(),
        ["exit Root_C", "enter Root_A", "enter Root_A_X"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root -> [Root_A -> Root_A_X, Root_P]"
    );
}

#[test]
fn a_panic_while_the_root_restarts_keeps_the_new_root() {
    use mid_transition::*;
    let mut chart = new_chart("");
    expect_panic(|| chart.trigger(Restart).unwrap());
    assert_eq!(
        take_log(),
        [
            "exit Root_A_X",
            "exit Root_A",
            "exit Root_P",
            "exit Root",
            "enter Root"
        ]
    );
    assert_eq!(chart.get_debug_name(), "Root -> [(empty), (empty)]");
    assert_eq!(chart.root().panic_in, "enter Root");
    assert!(chart.is_active::<Root>());
    assert!(!chart.is_active::<Root_A>());
    assert!(!chart.is_active::<Root_P>());
    assert!(chart.get_recursive_active_type_ids().is_empty());

    chart.root_mut().panic_in = "";
    chart.trigger(Reset).unwrap();
    assert_eq!(take_log(), ["enter Root_A", "enter Root_A_X"]);
    assert_eq!(
        chart.get_debug_name(),
        "Root -> [Root_A -> Root_A_X, (empty)]"
    );
}

#[test]
fn an_unlisted_target_requested_in_on_enter_during_a_transition_leaves_the_chart_consistent() {
    use mid_transition::*;
    let mut chart = new_chart("ghost");
    let message = expect_panic(|| chart.trigger(GoTo(To::B)).unwrap());
    assert!(message.starts_with("Transition target `validation::mid_transition::Root_Ghost`"));
    // The transition to Root_B had already started: like any panic in a hook while a
    // transition is applied.
    assert_eq!(take_log(), ["exit Root_A_X", "exit Root_A", "enter Root_B"]);
    assert_eq!(chart.get_debug_name(), "Root -> [Root_B, Root_P]");
    assert!(!chart.is_active::<Root_A>());
    assert!(chart.is_active::<Root_B>());
}

/// A restart (a transition to the root type) whose old root's own `on_exit` panics. The new root
/// has already taken the old one's place by then, but its `on_enter` has not run.
mod interrupted_restart {
    use super::*;

    // Root { generation, mode } -> Root_A (initial) | Root_B
    //
    // Every hook logs itself ("enter Root 1", "exit Root_A"). `mode` makes the root's hooks
    // misbehave: "panic in exit", "panic in enter", or "go to Root_B in enter" (its `on_enter`
    // requests Root_B). The root handles `Restart`, which requests a new root with the next
    // generation and no misbehaviour.

    pub struct Restart;
    impl Event for Restart {
        type Output = ();
    }

    pub struct Root {
        pub generation: u32,
        pub mode: &'static str,
    }
    impl std::fmt::Debug for Root {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Root {}", self.generation)
        }
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
            def.trigger::<Restart>();
        }
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            common::log("enter", &format!("Root {}", self.generation));
            match self.mode {
                "panic in enter" => panic!("enter Root panicked"),
                "go to Root_B in enter" => ctx.transition(Root_B),
                _ => {}
            }
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            common::log("exit", &format!("Root {}", self.generation));
            if self.mode == "panic in exit" {
                panic!("exit Root panicked");
            }
        }
        fn update(&mut self, _: &mut Context<Self>) {
            common::log("update", &format!("Root {}", self.generation));
        }
    }
    impl Trigger<Restart> for Root {
        fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
            ctx.transition(Root {
                generation: self.generation + 1,
                mode: "",
            });
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        logging_hooks!();
    }

    /// A chart whose restart from generation 0 to 1 was interrupted by a panic in generation
    /// 0's `on_exit`.
    pub fn interrupted() -> Statechart<Root> {
        let mut chart = Statechart::new(Root {
            generation: 0,
            mode: "panic in exit",
        });
        take_log();
        let message = mid_transition::expect_panic(|| chart.trigger(Restart).unwrap());
        assert_eq!(message, "exit Root panicked");
        assert_eq!(take_log(), ["exit Root_A", "exit Root 0"]);
        chart
    }
}

#[test]
fn a_panic_in_the_old_roots_on_exit_during_a_restart_still_leaves_a_root() {
    use interrupted_restart::*;
    let chart = interrupted();
    // The new root has taken the old one's place but is not entered yet: no initial children.
    assert_eq!(chart.root().generation, 1);
    assert!(chart.is_active::<Root>());
    assert!(!chart.is_active::<Root_A>());
    assert_eq!(chart.get_debug_name(), "Root 1 -> (empty)");
    assert!(chart.get_recursive_active_type_ids().is_empty());
}

#[test]
fn the_next_operation_first_enters_a_root_left_unentered_by_an_interrupted_restart() {
    use interrupted_restart::*;
    let entered = ["enter Root 1", "enter Root_A"];

    let mut chart = interrupted();
    chart.update();
    assert_eq!(
        take_log(),
        [&entered[..], &["update Root 1", "update Root_A"]].concat()
    );
    assert_eq!(chart.get_debug_name(), "Root 1 -> Root_A");

    // Never exited without having been entered.
    let restarted = ["exit Root_A", "exit Root 1", "enter Root 2", "enter Root_A"];
    let mut chart = interrupted();
    chart.trigger(Restart).unwrap();
    assert_eq!(take_log(), [&entered[..], &restarted].concat());
    assert_eq!(chart.get_debug_name(), "Root 2 -> Root_A");

    let mut chart = interrupted();
    assert_eq!(chart.trigger_all(Restart), Ok(vec![()]));
    assert_eq!(take_log(), [&entered[..], &restarted].concat());
    assert_eq!(chart.get_debug_name(), "Root 2 -> Root_A");
}

#[test]
fn a_root_left_unentered_is_entered_once_and_its_on_enter_requests_apply_first() {
    use interrupted_restart::*;
    let mut chart = interrupted();
    chart.root_mut().mode = "go to Root_B in enter";
    chart.update();
    assert_eq!(
        take_log(),
        [
            "enter Root 1",
            "enter Root_A",
            "exit Root_A",
            "enter Root_B",
            "update Root 1",
            "update Root_B"
        ]
    );
    chart.update();
    assert_eq!(take_log(), ["update Root 1", "update Root_B"]);
    assert_eq!(chart.get_debug_name(), "Root 1 -> Root_B");
}

#[test]
fn a_panic_while_entering_a_root_left_unentered_counts_as_entered() {
    use interrupted_restart::*;
    let mut chart = interrupted();
    chart.root_mut().mode = "panic in enter";
    let message = mid_transition::expect_panic(|| chart.update());
    assert_eq!(message, "enter Root panicked");
    assert_eq!(take_log(), ["enter Root 1"]);
    assert_eq!(chart.get_debug_name(), "Root 1 -> (empty)");

    // Not entered a second time; its empty region is filled by the next transition into it.
    chart.root_mut().mode = "";
    chart.update();
    assert_eq!(take_log(), ["update Root 1"]);
    chart.trigger(Restart).unwrap();
    assert_eq!(take_log(), ["exit Root 1", "enter Root 2", "enter Root_A"]);
}

/// Loop { remaining } -> Loop_A (initial) <-> Loop_B: each one's `on_enter` requests the other
/// while `remaining` is positive, counting it down. Building the chart with `remaining: n`
/// applies exactly `n` transitions in one batch. `Loop::update` requests `Loop_A`.
mod counted_loop {
    use super::*;

    #[derive(Debug)]
    pub struct Loop {
        pub remaining: u32,
    }
    impl State for Loop {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Loop_A>().child::<Loop_B>();
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Loop_A);
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_A;
    impl State for Loop_A {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().remaining > 0 {
                ctx.parent_mut().remaining -= 1;
                ctx.transition(Loop_B);
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_B;
    impl State for Loop_B {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().remaining > 0 {
                ctx.parent_mut().remaining -= 1;
                ctx.transition(Loop_A);
            }
        }
    }
}

#[test]
fn exactly_ten_thousand_transitions_in_one_batch_are_allowed() {
    use counted_loop::*;
    let chart = Statechart::new(Loop { remaining: 10_000 });
    assert_eq!(chart.root().remaining, 0);
    let panic = catch_unwind(|| Statechart::new(Loop { remaining: 10_001 })).unwrap_err();
    assert!(panic_message(panic).starts_with("more than 10000 transitions in one batch"));
}

#[test]
fn the_cycle_guard_counts_per_batch_not_per_chart() {
    use counted_loop::*;
    let mut chart = Statechart::new(Loop { remaining: 9_000 });
    for _ in 0..3 {
        // Loop_A again (a self-transition), then 9 000 more: 9 001 transitions per batch.
        chart.root_mut().remaining = 9_000;
        chart.update();
        assert_eq!(chart.root().remaining, 0);
    }
}

/// Requests that a panic discards, with targets that hold a resource.
mod discarded {
    use super::*;

    // Root { mode } -> Root_A (initial) | Root_B { lock } | Root_C
    //
    // `Lock` logs "release <name>" when it is dropped. `mode` picks the hook that requests
    // `Root_B { lock }` and then panics: "update" (Root_A's `update`) or "enter" (Root_C's
    // `on_enter`, while the transition to Root_C is applied). Root_A's `Boom` handler always does
    // it; its `GoToC` handler requests Root_C.

    #[derive(Debug)]
    pub struct Lock(pub &'static str);
    impl Drop for Lock {
        fn drop(&mut self) {
            common::log("release", self.0);
        }
    }

    pub struct Boom;
    impl Event for Boom {
        type Output = ();
    }

    pub struct GoToC;
    impl Event for GoToC {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub mode: &'static str,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>()
                .child_explicit::<Root_B>()
                .child_explicit::<Root_C>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Boom>().trigger::<GoToC>();
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().mode == "update" {
                ctx.transition(Root_B {
                    _lock: Lock("update"),
                });
                panic!("update panicked");
            }
        }
    }
    impl Trigger<Boom> for Root_A {
        fn handle(&mut self, _: &Boom, ctx: &mut Context<Self>) {
            ctx.transition(Root_B {
                _lock: Lock("handler"),
            });
            panic!("handler panicked");
        }
    }
    impl Trigger<GoToC> for Root_A {
        fn handle(&mut self, _: &GoToC, ctx: &mut Context<Self>) {
            ctx.transition(Root_C);
        }
    }

    #[derive(Debug)]
    pub struct Root_B {
        pub _lock: Lock,
    }
    impl State for Root_B {
        type Parent = Root;
    }

    #[derive(Debug)]
    pub struct Root_C;
    impl State for Root_C {
        type Parent = Root;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().mode == "enter" {
                ctx.transition(Root_B {
                    _lock: Lock("enter"),
                });
                panic!("enter panicked");
            }
        }
    }
}

#[test]
fn the_targets_of_requests_discarded_by_a_panic_are_dropped_while_it_unwinds() {
    use discarded::*;
    // A panic in `update`: the target requested before it is released before `catch_unwind`
    // returns, not at the next operation.
    let mut chart = Statechart::new(Root { mode: "update" });
    assert!(catch_unwind(AssertUnwindSafe(|| chart.update())).is_err());
    assert_eq!(take_log(), ["release update"]);
    assert_eq!(
        chart.get_debug_name(),
        "Root { mode: \"update\" } -> Root_A"
    );

    // A panic in a trigger handler.
    chart.root_mut().mode = "";
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger(Boom))).is_err());
    assert_eq!(take_log(), ["release handler"]);
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger_all(Boom))).is_err());
    assert_eq!(take_log(), ["release handler"]);
    assert!(chart.is_active::<Root_A>());

    // A panic in `on_enter` while a transition is applied: its request is part of the
    // interrupted batch.
    chart.root_mut().mode = "enter";
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger(GoToC))).is_err());
    assert_eq!(take_log(), ["release enter"]);
    assert_eq!(chart.get_debug_name(), "Root { mode: \"enter\" } -> Root_C");

    // None of them is applied later.
    chart.root_mut().mode = "";
    chart.update();
    assert_eq!(chart.get_debug_name(), "Root { mode: \"\" } -> Root_C");
    assert!(take_log().is_empty());
}

/// Two requests in one batch; applying the first one panics (in Root_B's `on_enter`). The
/// second one is "the rest of the batch".
mod interrupted_batch {
    use super::*;

    // Root { boom }
    // ├── region 0: Root_A (initial) | Root_B
    // └── region 1: Root_P (initial) | Root_Q

    pub struct Go;
    impl Event for Go {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub boom: bool,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
            def.region::<Root_P>().child::<Root_Q>();
            def.trigger::<Go>();
        }
    }
    impl Trigger<Go> for Root {
        fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) {
            ctx.transition(Root_B); // its on_enter panics while `boom`
            ctx.transition(Root_Q); // still queued when that panic hits
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().boom {
                panic!("enter Root_B panicked");
            }
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Root_A);
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_P;
    impl State for Root_P {
        type Parent = Root;
    }

    #[derive(Debug, Default)]
    pub struct Root_Q;
    impl State for Root_Q {
        type Parent = Root;
    }
}

#[test]
fn a_panic_while_a_batch_is_applied_discards_the_rest_of_the_batch_for_good() {
    use interrupted_batch::*;
    let mut chart = Statechart::new(Root { boom: true });
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger(Go))).is_err());
    // Root_B counts as entered; the queued request for Root_Q was not applied.
    assert_eq!(
        chart.get_debug_name(),
        "Root { boom: true } -> [Root_B, Root_P]"
    );

    // The next batch (Root_B's update requests Root_A) does not pick up the stale request,
    // although its requester (the root) is still active.
    chart.root_mut().boom = false;
    chart.update();
    assert_eq!(
        chart.get_debug_name(),
        "Root { boom: false } -> [Root_A, Root_P]"
    );
    assert!(!chart.is_active::<Root_Q>());
}

/// A restart interrupted by the old root's `on_exit` leaves the new root unentered. The next
/// operation enters it, its `on_enter` requests Root_B, and exiting Root_A for that request
/// panics.
mod pending_root {
    use super::*;

    // Root { generation, mode } -> Root_A (initial) | Root_B

    pub const LEAVE_A: &str = "request Root_B in on_enter; Root_A's on_exit panics";

    pub struct Restart;
    impl Event for Restart {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub generation: u32,
        pub mode: &'static str,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
            def.trigger::<Restart>();
        }
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if self.mode == LEAVE_A {
                ctx.transition(Root_B);
            }
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            if self.mode == "panic in exit" {
                panic!("exit Root panicked");
            }
        }
    }
    impl Trigger<Restart> for Root {
        fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
            ctx.transition(Root {
                generation: self.generation + 1,
                mode: "",
            });
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            if ctx.parent().mode == LEAVE_A {
                panic!("exit Root_A panicked");
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
    }
}

#[test]
fn a_panic_in_the_batch_that_enters_a_root_left_unentered_keeps_is_active_in_sync() {
    use pending_root::*;
    let mut chart = Statechart::new(Root {
        generation: 0,
        mode: "panic in exit",
    });
    assert!(catch_unwind(AssertUnwindSafe(|| chart.trigger(Restart))).is_err());
    chart.root_mut().mode = LEAVE_A;
    assert!(catch_unwind(AssertUnwindSafe(|| chart.update())).is_err());
    // Root_A was detached before its on_exit panicked, and Root_B was never entered.
    assert_eq!(
        chart.get_debug_name(),
        format!("Root {{ generation: 1, mode: {LEAVE_A:?} }} -> (empty)")
    );
    assert!(chart.get_recursive_active_type_ids().is_empty());
    assert!(chart.state::<Root_A>().is_none());
    assert!(!chart.is_active::<Root_A>());
    assert!(!chart.is_active::<Root_B>());
}

/// Like `counted_loop`, plus a request that is always dropped: `Loop_A::update` requests
/// `Loop_B`, but `Loop::update` (which runs first) requests `Loop_A`, and applying that exits
/// the `Loop_A` instance that made the second request.
mod counted_with_a_dropped_request {
    use super::*;

    #[derive(Debug)]
    pub struct Loop {
        pub remaining: u32,
    }
    impl State for Loop {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Loop_A>().child::<Loop_B>();
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Loop_A);
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_A;
    impl State for Loop_A {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().remaining > 0 {
                ctx.parent_mut().remaining -= 1;
                ctx.transition(Loop_B);
            }
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Loop_B); // always dropped
        }
    }

    #[derive(Debug, Default)]
    pub struct Loop_B;
    impl State for Loop_B {
        type Parent = Loop;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if ctx.parent().remaining > 0 {
                ctx.parent_mut().remaining -= 1;
                ctx.transition(Loop_A);
            }
        }
    }
}

#[test]
fn dropped_requests_do_not_count_towards_the_cycle_guard() {
    use counted_with_a_dropped_request::*;
    let mut chart = Statechart::new(Loop { remaining: 0 });
    // Loop_A again, then 9 999 more: 10 000 applied transitions and 1 dropped request.
    chart.root_mut().remaining = 9_999;
    chart.update();
    assert_eq!(chart.root().remaining, 0);
    assert!(chart.is_active::<Loop_B>());

    // One more applied transition does trip the guard.
    let mut chart = Statechart::new(Loop { remaining: 0 });
    chart.root_mut().remaining = 10_000;
    let panic = catch_unwind(AssertUnwindSafe(|| chart.update())).unwrap_err();
    assert!(panic_message(panic).starts_with("more than 10000 transitions in one batch"));
}
