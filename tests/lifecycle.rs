//! Lifecycle order (spec items 7 and 14): `on_enter` parent before child (initial children
//! included), `on_exit` deepest first, `update` parent before children, regions in
//! declaration order; constructors run when their region is entered (after the parent's
//! `on_enter` and its earlier regions, and during a transition after the old subtree is exited
//! and dropped); exited states are dropped right after their `on_exit`; the target of a dropped
//! request is dropped as soon as the request is.
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

mod save_on_exit {
    use super::*;

    // Game { saved }
    // └── region: Game_Play { points } (initial), Game_Over { score } (built from Game)
    //     Game_Over
    //     └── region: Game_Over_Menu (initial), Game_Over_Credits
    //
    // Game_Play's update requests Game_Over_Credits, so the transition builds the inactive
    // Game_Over with its constructor. Game_Play's on_exit saves its points into the parent.

    #[derive(Debug, Default)]
    pub struct Game {
        pub saved: u32,
    }
    impl State for Game {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Game_Play>().child_with(|game: &Game| {
                log("build", "Game_Over");
                Game_Over { score: game.saved }
            });
        }
    }

    #[derive(Debug, Default)]
    pub struct Game_Play {
        pub points: u32,
    }
    impl State for Game_Play {
        type Parent = Game;
        fn update(&mut self, ctx: &mut Context<Self>) {
            self.points = 7;
            ctx.transition(Game_Over_Credits);
        }
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            log("exit", "Game_Play");
            ctx.parent_mut().saved = self.points;
        }
    }
    impl Drop for Game_Play {
        fn drop(&mut self) {
            log("drop", "Game_Play");
        }
    }

    #[derive(Debug)]
    pub struct Game_Over {
        pub score: u32,
    }
    impl State for Game_Over {
        type Parent = Game;
        fn define(def: &mut Definition<Self>) {
            def.region::<Game_Over_Menu>().child::<Game_Over_Credits>();
        }
        fn on_enter(&mut self, _: &mut Context<Self>) {
            log("enter", "Game_Over");
        }
    }

    #[derive(Debug, Default)]
    pub struct Game_Over_Menu;
    impl State for Game_Over_Menu {
        type Parent = Game_Over;
    }

    #[derive(Debug, Default)]
    pub struct Game_Over_Credits;
    impl State for Game_Over_Credits {
        type Parent = Game_Over;
        fn on_enter(&mut self, _: &mut Context<Self>) {
            log("enter", "Game_Over_Credits");
        }
    }
}

/// A transition exits (and drops) the old subtree before it builds the target's inactive
/// ancestors, so their constructors see what the exited states' `on_exit` wrote into the
/// parent through `parent_mut`.
#[test]
fn a_transitions_constructor_runs_after_the_old_subtree_is_exited_and_dropped() {
    use save_on_exit::*;
    let mut chart = Statechart::new(Game::default());
    take_log();
    chart.update();
    assert_eq!(
        take_log(),
        [
            "exit Game_Play",
            "drop Game_Play",
            "build Game_Over",
            "enter Game_Over",
            "enter Game_Over_Credits"
        ]
    );
    assert_eq!(chart.root().saved, 7);
    assert_eq!(chart.state::<Game_Over>().unwrap().score, 7);
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

mod dropping_parallel {
    use super::*;

    // R -> [R_A, R_B -> R_B_1]: the earlier region is shallower than the later one. Every
    // state logs its `Drop`.

    macro_rules! logs_drop {
        ($($state:ident),*) => {
            $(impl Drop for $state {
                fn drop(&mut self) {
                    log("drop", stringify!($state));
                }
            })*
        };
    }
    logs_drop!(R, R_A, R_B, R_B_1);

    #[derive(Debug)]
    pub struct R;
    impl State for R {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_A>();
            def.region::<R_B>();
        }
    }

    #[derive(Debug, Default)]
    pub struct R_A;
    impl State for R_A {
        type Parent = R;
    }

    #[derive(Debug, Default)]
    pub struct R_B;
    impl State for R_B {
        type Parent = R;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_B_1>();
        }
    }

    #[derive(Debug, Default)]
    pub struct R_B_1;
    impl State for R_B_1 {
        type Parent = R_B;
    }
}

#[test]
fn dropping_the_chart_drops_the_states_in_post_order() {
    let chart = Statechart::new(dropping_parallel::R);
    take_log();
    drop(chart);
    // Children before their parent, regions in declaration order: R_A (depth 1) is dropped
    // before R_B_1 (depth 2).
    assert_eq!(take_log(), ["drop R_A", "drop R_B_1", "drop R_B", "drop R"]);
}

mod dropped_target {
    use super::*;

    // Root
    // ├── region 0: Root_A (initial), Root_B { lock } (explicit only), Root_C
    // └── region 1: Root_P (initial), Root_Q
    //
    // Update order: Root (-> Root_C, which exits Root_A), Root_A (-> Root_B { lock }: dropped,
    // because Root_A is exited first), Root_P (-> Root_Q).

    /// Logs when it is released.
    #[derive(Debug)]
    pub struct Lock;
    impl Drop for Lock {
        fn drop(&mut self) {
            log("release", "lock");
        }
    }

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>()
                .child_explicit::<Root_B>()
                .child::<Root_C>();
            def.region::<Root_P>().child::<Root_Q>();
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Root_C);
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Root_B { _lock: Lock });
        }
    }

    #[derive(Debug)]
    pub struct Root_B {
        pub _lock: Lock,
    }
    impl State for Root_B {
        type Parent = Root;
    }

    #[derive(Debug, Default)]
    pub struct Root_C;
    impl State for Root_C {
        type Parent = Root;
    }

    #[derive(Debug, Default)]
    pub struct Root_P;
    impl State for Root_P {
        type Parent = Root;
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Root_Q);
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_Q;
    impl State for Root_Q {
        type Parent = Root;
        logging_hooks!();
    }
}

/// The target of a request dropped by the drop rule (its requester was exited earlier in the
/// batch) is released right then, before the rest of the batch is applied.
#[test]
fn the_target_of_a_dropped_request_is_released_when_the_request_is_dropped() {
    use dropped_target::*;
    let mut chart = Statechart::new(Root);
    take_log();
    chart.update();
    assert_eq!(take_log_without_updates(), ["release lock", "enter Root_Q"]);
    assert_eq!(chart.get_debug_name(), "Root -> [Root_C, Root_Q]");
}
