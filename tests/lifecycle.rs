//! Lifecycle order (spec items 7 and 14): `on_enter` parent before child (initial children
//! included), `on_exit` deepest first, `update` parent before children, regions in
//! declaration order; constructors run after the parent's `on_enter`.
#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use common::{log, take_log, take_log_without_updates};
use statechart::prelude::*;

// Root
// └── region: Root_A (initial), Root_B
//     Root_A
//     ├── region 0: Root_A_X (initial)
//     │   Root_A_X
//     │   └── region: Root_A_X_Deep (initial)
//     └── region 1: Root_A_P (initial)

/// Handled by `Root`: -> `Root_B`.
struct GoToB;
impl Event for GoToB {
    type Output = ();
}

#[derive(Debug)]
struct Root;
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A>().child::<Root_B>();
        def.trigger::<GoToB>();
    }
    logging_hooks!();
}
impl Trigger<GoToB> for Root {
    fn handle(&mut self, _: &GoToB, ctx: &mut Context<Self>) {
        ctx.transition(Root_B);
    }
}

#[derive(Debug, Default)]
struct Root_A;
impl State for Root_A {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A_X>();
        def.region::<Root_A_P>();
    }
    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        log("enter", "Root_A");
        // Active from just before on_enter; children are entered after it.
        assert!(ctx.is_active::<Root_A>());
        assert!(!ctx.is_active::<Root_A_X>());
    }
    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        log("exit", "Root_A");
        // Still active during on_exit; children were exited before it.
        assert!(ctx.is_active::<Root_A>());
        assert!(!ctx.is_active::<Root_A_X>());
        assert!(!ctx.is_active::<Root_B>());
    }
    fn update(&mut self, _: &mut Context<Self>) {
        log("update", "Root_A");
    }
}

#[derive(Debug, Default)]
struct Root_A_X;
impl State for Root_A_X {
    type Parent = Root_A;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A_X_Deep>();
    }
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_A_X_Deep;
impl State for Root_A_X_Deep {
    type Parent = Root_A_X;
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_A_P;
impl State for Root_A_P {
    type Parent = Root_A;
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_B;
impl State for Root_B {
    type Parent = Root;
    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        log("enter", "Root_B");
        // The old sibling is gone before the new one is entered.
        assert!(!ctx.is_active::<Root_A>());
    }
}

#[test]
fn construction_enters_the_root_then_every_initial_descendant_top_down() {
    let chart = Statechart::new(Root);
    assert_eq!(
        take_log(),
        [
            "enter Root",
            "enter Root_A",
            "enter Root_A_X",
            "enter Root_A_X_Deep",
            "enter Root_A_P"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root -> Root_A -> [Root_A_X -> Root_A_X_Deep, Root_A_P]"
    );
}

#[test]
fn update_runs_parents_before_children_and_regions_in_declaration_order() {
    let mut chart = Statechart::new(Root);
    take_log();
    chart.update();
    assert_eq!(
        take_log(),
        [
            "update Root",
            "update Root_A",
            "update Root_A_X",
            "update Root_A_X_Deep",
            "update Root_A_P"
        ]
    );
}

#[test]
fn exit_runs_deepest_first_with_regions_in_declaration_order_then_enter_runs_top_down() {
    let mut chart = Statechart::new(Root);
    take_log();
    chart.trigger(GoToB).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_X_Deep",
            "exit Root_A_X",
            "exit Root_A_P",
            "exit Root_A",
            "enter Root_B"
        ]
    );
}

mod seeded {
    use super::*;

    // Seed { base } -> Seed_Mid { seed } -> Seed_Mid_Leaf { seed }

    #[derive(Debug)]
    pub struct Seed {
        pub base: u32,
    }
    impl State for Seed {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region_with(|seed: &Seed| Seed_Mid { seed: seed.base });
        }
    }

    #[derive(Debug)]
    pub struct Seed_Mid {
        pub seed: u32,
    }
    impl State for Seed_Mid {
        type Parent = Seed;
        fn define(def: &mut Definition<Self>) {
            def.region_with(|mid: &Seed_Mid| Seed_Mid_Leaf { seed: mid.seed });
        }
        fn on_enter(&mut self, _: &mut Context<Self>) {
            self.seed *= 10;
        }
    }

    #[derive(Debug)]
    pub struct Seed_Mid_Leaf {
        pub seed: u32,
    }
    impl State for Seed_Mid_Leaf {
        type Parent = Seed_Mid;
    }
}

#[test]
fn a_constructor_sees_the_parent_data_as_on_enter_left_it() {
    let chart = Statechart::new(seeded::Seed { base: 7 });
    assert_eq!(
        chart.get_debug_name(),
        "Seed { base: 7 } -> Seed_Mid { seed: 70 } -> Seed_Mid_Leaf { seed: 70 }"
    );
    assert_eq!(chart.state::<seeded::Seed_Mid_Leaf>().unwrap().seed, 70);
}
