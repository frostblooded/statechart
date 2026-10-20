//! Transition shapes: sibling, into inactive ancestors (built from parent data), two levels
//! below an active non-root LCA, deep to shallow with a payload, self-transition, to an active
//! ancestor, and to the root (including the requests of the new root's `on_enter`).
#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use common::take_log_without_updates;
use statechart::prelude::*;

// Root { generation }
// └── region: Root_A (initial), Root_B
//     Root_A
//     ├── region 0: Root_A_X (initial), Root_A_Y
//     └── region 1: Root_A_P (initial)
//     Root_B
//     └── region: Root_B_X (initial)

/// Handled by `Root`: requests a transition to the named state.
enum Go {
    RootA,
    RootB,
    RootAX,
    RootAY,
    Root,
}
impl Event for Go {
    type Output = ();
}

/// Handled by `Root_A_X`: requests its own parent, `Root_A`.
struct ReenterParent;
impl Event for ReenterParent {
    type Output = ();
}

#[derive(Debug)]
struct Root {
    generation: u32,
}
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A>().child::<Root_B>();
        def.trigger::<Go>();
    }
    logging_hooks!();
}
impl Trigger<Go> for Root {
    fn handle(&mut self, go: &Go, ctx: &mut Context<Self>) {
        match go {
            Go::RootA => ctx.transition(Root_A),
            Go::RootB => ctx.transition(Root_B),
            Go::RootAX => ctx.transition(Root_A_X::default()),
            Go::RootAY => ctx.transition(Root_A_Y),
            Go::Root => ctx.transition(Root {
                generation: self.generation + 1,
            }),
        }
    }
}

#[derive(Debug, Default)]
struct Root_A;
impl State for Root_A {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A_X>().child::<Root_A_Y>();
        def.region::<Root_A_P>();
    }
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_A_X {
    updates: u32,
}
impl State for Root_A_X {
    type Parent = Root_A;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<ReenterParent>();
    }
    fn on_enter(&mut self, _: &mut Context<Self>) {
        common::log("enter", "Root_A_X");
    }
    fn on_exit(&mut self, _: &mut ExitContext<Self>) {
        common::log("exit", "Root_A_X");
    }
    fn update(&mut self, _: &mut Context<Self>) {
        self.updates += 1;
    }
}
impl Trigger<ReenterParent> for Root_A_X {
    fn handle(&mut self, _: &ReenterParent, ctx: &mut Context<Self>) {
        ctx.transition(Root_A);
    }
}

#[derive(Debug, Default)]
struct Root_A_Y;
impl State for Root_A_Y {
    type Parent = Root_A;
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
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_B_X>();
    }
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_B_X;
impl State for Root_B_X {
    type Parent = Root_B;
    logging_hooks!();
}

fn new_chart() -> Statechart<Root> {
    let chart = Statechart::new(Root { generation: 0 });
    common::take_log();
    chart
}

#[test]
fn a_sibling_transition_exits_the_old_subtree_and_enters_the_target_with_its_initial_children() {
    let mut chart = new_chart();
    chart.trigger(Go::RootB).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_X",
            "exit Root_A_P",
            "exit Root_A",
            "enter Root_B",
            "enter Root_B_X"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { generation: 0 } -> Root_B -> Root_B_X"
    );
}

#[test]
fn a_deep_target_enters_its_inactive_ancestor_and_the_ancestors_other_regions() {
    let mut chart = new_chart();
    chart.trigger(Go::RootB).unwrap();
    common::take_log();
    chart.trigger(Go::RootAY).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_B_X",
            "exit Root_B",
            "enter Root_A",
            "enter Root_A_Y",
            "enter Root_A_P"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { generation: 0 } -> Root_A -> [Root_A_Y, Root_A_P]"
    );
}

#[test]
fn a_self_transition_exits_and_re_enters_the_whole_subtree() {
    let mut chart = new_chart();
    chart.trigger(Go::RootA).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_X",
            "exit Root_A_P",
            "exit Root_A",
            "enter Root_A",
            "enter Root_A_X",
            "enter Root_A_P"
        ]
    );
}

// User decision D4: a transition into one region of an active parallel state changes only that
// region; the parallel state and its other regions are not exited. Targeting the parallel state
// itself resets every one of its regions.

#[test]
fn a_transition_into_one_region_of_an_active_parallel_state_changes_only_that_region() {
    let mut chart = new_chart();
    chart.trigger(Go::RootAY).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Root_A_X", "enter Root_A_Y"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { generation: 0 } -> Root_A -> [Root_A_Y, Root_A_P]"
    );
}

#[test]
fn targeting_the_parallel_state_itself_resets_every_region() {
    let mut chart = new_chart();
    chart.trigger(Go::RootAY).unwrap();
    common::take_log();
    chart.trigger(Go::RootA).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_Y",
            "exit Root_A_P",
            "exit Root_A",
            "enter Root_A",
            "enter Root_A_X",
            "enter Root_A_P"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { generation: 0 } -> Root_A -> [Root_A_X { updates: 0 }, Root_A_P]"
    );
}

#[test]
fn a_leaf_self_transition_replaces_the_instance_and_leaves_other_regions_alone() {
    let mut chart = new_chart();
    chart.update();
    assert_eq!(chart.state::<Root_A_X>().unwrap().updates, 1);
    common::take_log();
    chart.trigger(Go::RootAX).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Root_A_X", "enter Root_A_X"]
    );
    assert_eq!(chart.state::<Root_A_X>().unwrap().updates, 0);
}

#[test]
fn a_transition_to_an_active_ancestor_re_enters_that_ancestor() {
    let mut chart = new_chart();
    chart.trigger(ReenterParent).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_X",
            "exit Root_A_P",
            "exit Root_A",
            "enter Root_A",
            "enter Root_A_X",
            "enter Root_A_P"
        ]
    );
}

mod below_the_lca {
    use super::*;

    // R
    // └── region: R_Z (initial)
    //     R_Z
    //     └── region: R_Z_1 (initial), R_Z_2
    //         R_Z_2
    //         └── region: R_Z_2_A (initial), R_Z_2_B
    //
    // R_Z_1 handles `Deep` (-> R_Z_2_B, a grandchild of its active parent R_Z).

    pub struct Deep;
    impl Event for Deep {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct R;
    impl State for R {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_Z>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_Z;
    impl State for R_Z {
        type Parent = R;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_Z_1>().child::<R_Z_2>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_Z_1;
    impl State for R_Z_1 {
        type Parent = R_Z;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Deep>();
        }
        logging_hooks!();
    }
    impl Trigger<Deep> for R_Z_1 {
        fn handle(&mut self, _: &Deep, ctx: &mut Context<Self>) {
            ctx.transition(R_Z_2_B);
        }
    }

    #[derive(Debug, Default)]
    pub struct R_Z_2;
    impl State for R_Z_2 {
        type Parent = R_Z;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_Z_2_A>().child::<R_Z_2_B>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_Z_2_A;
    impl State for R_Z_2_A {
        type Parent = R_Z_2;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_Z_2_B;
    impl State for R_Z_2_B {
        type Parent = R_Z_2;
        logging_hooks!();
    }
}

/// The LCA is the deepest active state on the target's path, also when the target is two
/// levels below it: `R_Z` stays, only its region is exited and entered.
#[test]
fn a_transition_two_levels_below_an_active_non_root_state_does_not_re_enter_it() {
    use below_the_lca::*;
    let mut chart = Statechart::new(R);
    common::take_log();
    chart.trigger(Deep).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit R_Z_1", "enter R_Z_2", "enter R_Z_2_B"]
    );
    assert_eq!(chart.get_debug_name(), "R -> R_Z -> R_Z_2 -> R_Z_2_B");
}

#[test]
fn a_transition_to_the_root_type_restarts_the_whole_chart() {
    let mut chart = new_chart();
    chart.trigger(Go::Root).unwrap();
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A_X",
            "exit Root_A_P",
            "exit Root_A",
            "exit Root",
            "enter Root",
            "enter Root_A",
            "enter Root_A_X",
            "enter Root_A_P"
        ]
    );
    assert_eq!(chart.root().generation, 1);
}

mod cross_level {
    use super::*;

    // Game { level, boss_hp }
    // └── region: Game_Menu (initial), Game_Play (built from Game), Game_Over (explicit only)
    //     Game_Play { level }
    //     └── region: Game_Play_Running (initial), Game_Play_Boss (built from Game_Play)
    //
    // Game_Menu.update jumps straight into Game_Play_Boss; Game_Play is not active, so the
    // library builds it from `Game` with the constructor registered by `child_with`.

    #[derive(Debug)]
    pub struct Game {
        pub level: u32,
        pub boss_hp: u32,
    }
    impl State for Game {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Game_Menu>()
                .child_with(|game: &Game| Game_Play { level: game.level })
                .child_explicit::<Game_Over>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Game_Menu;
    impl State for Game_Menu {
        type Parent = Game;
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Game_Play_Boss { hp: 0 });
        }
    }

    #[derive(Debug)]
    pub struct Game_Play {
        pub level: u32,
    }
    impl State for Game_Play {
        type Parent = Game;
        fn define(def: &mut Definition<Self>) {
            def.region::<Game_Play_Running>()
                .child_with(|play: &Game_Play| Game_Play_Boss { hp: play.level });
        }
    }

    #[derive(Debug, Default)]
    pub struct Game_Play_Running;
    impl State for Game_Play_Running {
        type Parent = Game_Play;
    }

    #[derive(Debug)]
    pub struct Game_Play_Boss {
        pub hp: u32,
    }
    impl State for Game_Play_Boss {
        type Parent = Game_Play;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            // The implicitly built parent is already entered and reachable.
            self.hp += ctx.parent().level * 10;
            ctx.ancestor_mut::<Game>().unwrap().boss_hp = self.hp;
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            // Deep -> shallow, with a payload the target cannot be built without.
            ctx.transition(Game_Over { score: self.hp + 1 });
        }
    }

    /// Only reachable through `transition(Game_Over { score })`: no constructor.
    #[derive(Debug)]
    pub struct Game_Over {
        pub score: u32,
    }
    impl State for Game_Over {
        type Parent = Game;
    }
}

#[test]
fn a_deep_target_builds_its_inactive_ancestor_from_the_parents_data() {
    use cross_level::*;
    let mut chart = Statechart::new(Game {
        level: 3,
        boss_hp: 0,
    });
    assert_eq!(
        chart.get_debug_name(),
        "Game { level: 3, boss_hp: 0 } -> Game_Menu"
    );
    chart.update();
    // Game_Play { level: 3 } came from Game's constructor; the target instance (hp: 0) is the
    // one passed to `transition`, not one built by Game_Play's constructor.
    assert_eq!(
        chart.get_debug_name(),
        "Game { level: 3, boss_hp: 30 } -> Game_Play { level: 3 } -> Game_Play_Boss { hp: 30 }"
    );
}

#[test]
fn a_deep_state_can_transition_to_a_shallow_target_that_carries_data() {
    use cross_level::*;
    let mut chart = Statechart::new(Game {
        level: 2,
        boss_hp: 0,
    });
    chart.update();
    chart.update();
    assert!(chart.is_active::<Game_Over>());
    assert!(!chart.is_active::<Game_Play>());
    assert!(!chart.is_active::<Game_Play_Boss>());
    assert_eq!(chart.state::<Game_Over>().unwrap().score, 21);
}

mod restart {
    use super::*;

    // Root { generation } -> Root_A (initial) | Root_B
    //
    // Root handles `Restart` (-> a new root with the next generation) and `GoB` (-> Root_B).
    // Root's `on_enter` logs whether it counts as active; Root_A's `on_exit` logs which root
    // generation its parent is.

    pub struct Restart;
    impl Event for Restart {
        type Output = ();
    }

    pub struct GoB;
    impl Event for GoB {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub generation: u32,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
            def.trigger::<Restart>().trigger::<GoB>();
        }
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            common::log(
                "enter",
                &format!(
                    "Root {} (active: {})",
                    self.generation,
                    ctx.is_active::<Root>()
                ),
            );
        }
    }
    impl Trigger<Restart> for Root {
        fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
            ctx.transition(Root {
                generation: self.generation + 1,
            });
        }
    }
    impl Trigger<GoB> for Root {
        fn handle(&mut self, _: &GoB, ctx: &mut Context<Self>) {
            ctx.transition(Root_B);
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            common::log(
                "exit",
                &format!("Root_A (parent generation {})", ctx.parent().generation),
            );
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
    }
}

#[test]
fn after_a_restart_the_new_root_is_active_and_its_requests_apply() {
    use restart::*;
    let mut chart = Statechart::new(Root { generation: 0 });
    take_log_without_updates();
    chart.trigger(Restart).unwrap();
    // The old root's descendants are exited under the old root, and the new root is live.
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Root_A (parent generation 0)",
            "enter Root 1 (active: true)"
        ]
    );
    assert!(chart.is_active::<Root>());
    chart.trigger(GoB).unwrap();
    assert_eq!(chart.get_debug_name(), "Root { generation: 1 } -> Root_B");
}

mod restart_requests {
    use super::*;

    // Root { generation } -> Root_A (initial) | Root_B
    //
    // Root handles `Restart` (-> a new root with the next generation). The new root's `on_enter`
    // (generation 1) and Root_A's `on_enter` (generation 2) request transitions while the
    // restart enters them.

    pub struct Restart;
    impl Event for Restart {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub generation: u32,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child::<Root_B>();
            def.trigger::<Restart>();
        }
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            if self.generation == 1 {
                ctx.transition(Root_B);
            }
        }
    }
    impl Trigger<Restart> for Root {
        fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
            ctx.transition(Root {
                generation: self.generation + 1,
            });
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            common::log("enter", "Root_A");
            if ctx.parent().generation == 2 {
                ctx.transition(Root_B);
            }
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            common::log("exit", "Root_A");
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        logging_hooks!();
    }
}

/// The requests that `on_enter` hooks make while a restart enters the new root and its
/// initial descendants are applied, like those of any other transition.
#[test]
fn on_enter_requests_made_while_a_restart_enters_the_new_root_are_applied() {
    use restart_requests::*;
    let mut chart = Statechart::new(Root { generation: 0 });
    common::take_log();
    chart.trigger(Restart).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Root_A", "enter Root_A", "exit Root_A", "enter Root_B"]
    );
    assert_eq!(chart.get_debug_name(), "Root { generation: 1 } -> Root_B");

    // Generation 2: this time the new root's initial child makes the request.
    chart.trigger(Restart).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Root_B", "enter Root_A", "exit Root_A", "enter Root_B"]
    );
    assert_eq!(chart.get_debug_name(), "Root { generation: 2 } -> Root_B");
}
