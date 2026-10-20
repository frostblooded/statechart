//! Lifecycle order (spec items 7 and 14): `on_enter` parent before child (initial children
//! included), `on_exit` deepest first, `update` parent before children, regions in
//! declaration order; constructors run when their region is entered (after the parent's
//! `on_enter` and its earlier regions); exited states are dropped right after their `on_exit`.
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

mod hub {
    use super::*;

    // Hub { n }
    // ├── region 0: Hub_Left (initial), Hub_Deep { seen } (built from Hub)
    // │   Hub_Deep
    // │   └── region: Hub_Deep_Leaf (initial)
    // └── region 1: Hub_Right { seen } (initial, built from Hub)

    #[derive(Debug)]
    pub struct Hub {
        pub n: u32,
    }
    impl State for Hub {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Hub_Left>()
                .child_with(|hub: &Hub| Hub_Deep { seen: hub.n });
            def.region_with(|hub: &Hub| Hub_Right { seen: hub.n });
        }
        fn on_enter(&mut self, _: &mut Context<Self>) {
            self.n = 10;
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            self.n = 42;
            ctx.transition(Hub_Deep_Leaf);
        }
    }

    #[derive(Debug, Default)]
    pub struct Hub_Left;
    impl State for Hub_Left {
        type Parent = Hub;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            ctx.parent_mut().n += 5;
        }
    }

    #[derive(Debug)]
    pub struct Hub_Deep {
        pub seen: u32,
    }
    impl State for Hub_Deep {
        type Parent = Hub;
        fn define(def: &mut Definition<Self>) {
            def.region::<Hub_Deep_Leaf>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Hub_Deep_Leaf;
    impl State for Hub_Deep_Leaf {
        type Parent = Hub_Deep;
    }

    #[derive(Debug)]
    pub struct Hub_Right {
        pub seen: u32,
    }
    impl State for Hub_Right {
        type Parent = Hub;
    }
}

#[test]
fn a_constructor_in_a_later_region_sees_what_earlier_regions_on_enter_did_to_the_parent() {
    let chart = Statechart::new(hub::Hub { n: 0 });
    // Hub's on_enter set 10, then region 0 (Hub_Left's on_enter) added 5 before region 1's
    // constructor ran.
    assert_eq!(
        chart.get_debug_name(),
        "Hub { n: 15 } -> [Hub_Left, Hub_Right { seen: 15 }]"
    );
    assert_eq!(chart.state::<hub::Hub_Right>().unwrap().seen, 15);
}

#[test]
fn a_constructor_run_by_a_transition_sees_the_parents_current_data() {
    let mut chart = Statechart::new(hub::Hub { n: 0 });
    chart.update();
    // Hub_Deep is built when the transition is applied, long after Hub's on_enter.
    assert_eq!(
        chart.get_debug_name(),
        "Hub { n: 42 } -> [Hub_Deep { seen: 42 } -> Hub_Deep_Leaf, Hub_Right { seen: 15 }]"
    );
    assert_eq!(chart.state::<hub::Hub_Deep>().unwrap().seen, 42);
}

mod dropping {
    use super::*;

    // Root -> Root_A (initial) | Root_B; Root_A -> Root_A_X. Every state logs its hooks and
    // its `Drop`.

    /// Handled by `Root`: -> `Root_B` (a self-transition when `Root_B` is active).
    pub struct GoToB;
    impl Event for GoToB {
        type Output = ();
    }

    macro_rules! logs_drop {
        ($($state:ident),*) => {
            $(impl Drop for $state {
                fn drop(&mut self) {
                    log("drop", stringify!($state));
                }
            })*
        };
    }
    logs_drop!(Root, Root_A, Root_A_X, Root_B);

    #[derive(Debug)]
    pub struct Root;
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
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A_X>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Root_A_X;
    impl State for Root_A_X {
        type Parent = Root_A;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        logging_hooks!();
    }
}

#[test]
fn exited_states_are_dropped_right_after_their_on_exit_before_anything_is_entered() {
    let mut chart = Statechart::new(dropping::Root);
    take_log();
    chart.trigger(dropping::GoToB).unwrap();
    assert_eq!(
        take_log(),
        [
            "exit Root_A_X",
            "drop Root_A_X",
            "exit Root_A",
            "drop Root_A",
            "enter Root_B"
        ]
    );

    // An external self-transition: the old instance is gone before the new one's on_enter, so
    // the new one can take over whatever the old one held (a socket, a file lock, ...).
    chart.trigger(dropping::GoToB).unwrap();
    assert_eq!(take_log(), ["exit Root_B", "drop Root_B", "enter Root_B"]);
}

#[test]
fn dropping_the_chart_drops_every_state_deepest_first_without_on_exit() {
    let chart = Statechart::new(dropping::Root);
    take_log();
    drop(chart);
    assert_eq!(take_log(), ["drop Root_A_X", "drop Root_A", "drop Root"]);
}
