//! Parallel regions: initial states, independence, per-region trigger dispatch (also with an
//! unregistered handler in one region), a shared ancestor running at most once, cross-region
//! transitions (also into an inactive state below the other region) and "in state" guards.
#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use common::take_log_without_updates;
use statechart::prelude::*;

// Player has two regions running in parallel:
//   region 0 (movement): Player_Idle (initial), Player_Walking
//   region 1 (weapon):   Player_Holstered (initial), Player_Aiming

/// Handled only by `Player_Aiming`.
struct Fire;
impl Event for Fire {
    type Output = &'static str;
}

/// Handled by `Player_Idle` and `Player_Holstered` (one per region).
struct Ping;
impl Event for Ping {
    type Output = &'static str;
}

/// Handled by `Player`, `Player_Walking` and `Player_Aiming`.
struct Tick;
impl Event for Tick {
    type Output = &'static str;
}

/// `Player_Idle` -> `Player_Walking`.
struct Walk;
impl Event for Walk {
    type Output = ();
}

/// Handled by `Player_Holstered`, which moves the *other* region: -> `Player_Walking`.
struct WalkFromWeapon;
impl Event for WalkFromWeapon {
    type Output = ();
}

/// `Player_Holstered` -> `Player_Aiming`.
struct Aim;
impl Event for Aim {
    type Output = ();
}

/// `Player_Walking` answers whether the weapon region is aiming (an "in state" guard).
struct CanShootWhileWalking;
impl Event for CanShootWhileWalking {
    type Output = bool;
}

/// Handled by `Player_Holstered`. `Player_Idle` implements `Trigger<Shout>` but never registers
/// it, and `Player` does not handle it.
struct Shout;
impl Event for Shout {
    type Output = &'static str;
}

#[derive(Debug, Default)]
struct Player {
    ticks_handled: u32,
}
impl State for Player {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Player_Idle>().child::<Player_Walking>();
        def.region::<Player_Holstered>().child::<Player_Aiming>();
        def.trigger::<Tick>();
    }
    logging_hooks!();
}
impl Trigger<Tick> for Player {
    fn handle(&mut self, _: &Tick, _: &mut Context<Self>) -> &'static str {
        self.ticks_handled += 1;
        "Player"
    }
}

#[derive(Debug, Default)]
struct Player_Idle;
impl State for Player_Idle {
    type Parent = Player;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Ping>().trigger::<Walk>();
    }
    logging_hooks!();
}
impl Trigger<Ping> for Player_Idle {
    fn handle(&mut self, _: &Ping, _: &mut Context<Self>) -> &'static str {
        "Player_Idle"
    }
}
impl Trigger<Walk> for Player_Idle {
    fn handle(&mut self, _: &Walk, ctx: &mut Context<Self>) {
        ctx.transition(Player_Walking);
    }
}
// Deliberately NOT registered in `Player_Idle::define`.
impl Trigger<Shout> for Player_Idle {
    fn handle(&mut self, _: &Shout, _: &mut Context<Self>) -> &'static str {
        panic!("an unregistered handler must never run");
    }
}

#[derive(Debug, Default)]
struct Player_Walking;
impl State for Player_Walking {
    type Parent = Player;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Tick>().trigger::<CanShootWhileWalking>();
    }
    logging_hooks!();
}
impl Trigger<Tick> for Player_Walking {
    fn handle(&mut self, _: &Tick, _: &mut Context<Self>) -> &'static str {
        "Player_Walking"
    }
}
impl Trigger<CanShootWhileWalking> for Player_Walking {
    fn handle(&mut self, _: &CanShootWhileWalking, ctx: &mut Context<Self>) -> bool {
        ctx.is_active::<Player_Aiming>()
    }
}

#[derive(Debug, Default)]
struct Player_Holstered;
impl State for Player_Holstered {
    type Parent = Player;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Ping>()
            .trigger::<Aim>()
            .trigger::<WalkFromWeapon>()
            .trigger::<Shout>();
    }
    logging_hooks!();
}
impl Trigger<Ping> for Player_Holstered {
    fn handle(&mut self, _: &Ping, _: &mut Context<Self>) -> &'static str {
        "Player_Holstered"
    }
}
impl Trigger<Shout> for Player_Holstered {
    fn handle(&mut self, _: &Shout, _: &mut Context<Self>) -> &'static str {
        "Player_Holstered"
    }
}
impl Trigger<Aim> for Player_Holstered {
    fn handle(&mut self, _: &Aim, ctx: &mut Context<Self>) {
        ctx.transition(Player_Aiming);
    }
}
impl Trigger<WalkFromWeapon> for Player_Holstered {
    fn handle(&mut self, _: &WalkFromWeapon, ctx: &mut Context<Self>) {
        ctx.transition(Player_Walking);
    }
}

#[derive(Debug, Default)]
struct Player_Aiming;
impl State for Player_Aiming {
    type Parent = Player;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Fire>().trigger::<Tick>();
    }
    logging_hooks!();
}
impl Trigger<Fire> for Player_Aiming {
    fn handle(&mut self, _: &Fire, _: &mut Context<Self>) -> &'static str {
        "bang"
    }
}
impl Trigger<Tick> for Player_Aiming {
    fn handle(&mut self, _: &Tick, _: &mut Context<Self>) -> &'static str {
        "Player_Aiming"
    }
}

fn new_player() -> Statechart<Player> {
    let chart = Statechart::new(Player::default());
    common::take_log();
    chart
}

#[test]
fn every_region_starts_in_its_initial_state() {
    let chart = Statechart::new(Player::default());
    assert_eq!(
        common::take_log(),
        [
            "enter Player",
            "enter Player_Idle",
            "enter Player_Holstered"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Player { ticks_handled: 0 } -> [Player_Idle, Player_Holstered]"
    );
    assert!(chart.is_active::<Player_Idle>());
    assert!(chart.is_active::<Player_Holstered>());
    assert!(!chart.is_active::<Player_Walking>());
    assert!(!chart.is_active::<Player_Aiming>());
}

#[test]
fn update_visits_every_region_in_declaration_order() {
    let mut chart = new_player();
    chart.update();
    assert_eq!(
        common::take_log(),
        [
            "update Player",
            "update Player_Idle",
            "update Player_Holstered"
        ]
    );
}

#[test]
fn a_transition_in_one_region_leaves_the_other_alone() {
    let mut chart = new_player();
    chart.trigger(Walk).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Player_Idle", "enter Player_Walking"]
    );
    chart.trigger(Aim).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Player_Holstered", "enter Player_Aiming"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Player { ticks_handled: 0 } -> [Player_Walking, Player_Aiming]"
    );
}

#[test]
fn a_cross_region_request_changes_only_the_target_region() {
    let mut chart = new_player();
    // Requested from region 1, targets region 0. Neither region 1 nor Player is re-entered
    // (user decision D4: unlike SCXML, the parallel parent is not reset).
    chart.trigger(WalkFromWeapon).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit Player_Idle", "enter Player_Walking"]
    );
    assert!(chart.is_active::<Player_Holstered>());
    assert_eq!(
        chart.get_debug_name(),
        "Player { ticks_handled: 0 } -> [Player_Walking, Player_Holstered]"
    );
}

#[test]
fn every_region_that_can_handle_a_trigger_does() {
    let mut chart = new_player();
    assert_eq!(
        chart.trigger_all(Ping),
        Ok(vec!["Player_Idle", "Player_Holstered"])
    );
    // `trigger` runs the same dispatch (both handlers) and returns the first output.
    assert_eq!(chart.trigger(Ping), Ok("Player_Idle"));
}

#[test]
fn a_trigger_is_not_handled_only_if_no_branch_handles_it() {
    let mut chart = new_player();
    assert_eq!(chart.trigger(Fire), Err(NotHandled));
    assert_eq!(chart.trigger_all(Fire), Err(NotHandled));
    chart.trigger(Aim).unwrap();
    assert_eq!(chart.trigger(Fire), Ok("bang"));
    assert_eq!(chart.trigger_all(Fire), Ok(vec!["bang"]));
}

/// User decision D2 with parallel regions: `Player_Idle`'s unregistered handler never runs and
/// no ancestor of it handles `Shout`, but the other region does, so the trigger is handled.
#[test]
fn an_unregistered_handler_in_one_region_is_skipped_while_another_region_handles_it() {
    let mut chart = new_player();
    assert_eq!(chart.trigger(Shout), Ok("Player_Holstered"));
    assert_eq!(chart.trigger_all(Shout), Ok(vec!["Player_Holstered"]));
}

#[test]
fn a_shared_ancestor_runs_once_and_only_for_unhandled_branches() {
    let mut chart = new_player();

    // Neither branch handles Tick below Player: Player runs once, on behalf of both.
    assert_eq!(chart.trigger_all(Tick), Ok(vec!["Player"]));
    assert_eq!(chart.root().ticks_handled, 1);

    // The movement branch handles it; the weapon branch bubbles up to Player.
    chart.trigger(Walk).unwrap();
    assert_eq!(
        chart.trigger_all(Tick),
        Ok(vec!["Player_Walking", "Player"])
    );
    assert_eq!(chart.root().ticks_handled, 2);

    // Both branches handle it below Player: Player does not run.
    chart.trigger(Aim).unwrap();
    assert_eq!(
        chart.trigger_all(Tick),
        Ok(vec!["Player_Walking", "Player_Aiming"])
    );
    assert_eq!(chart.root().ticks_handled, 2);
}

#[test]
fn a_state_can_check_whether_a_state_in_another_region_is_active() {
    let mut chart = new_player();
    chart.trigger(Walk).unwrap();
    assert_eq!(chart.trigger(CanShootWhileWalking), Ok(false));
    chart.trigger(Aim).unwrap();
    assert_eq!(chart.trigger(CanShootWhileWalking), Ok(true));
}

// Transitions requested by a handler are applied after the whole dispatch: the other region's
// handler still runs on the state that was active when the trigger was fired.
mod deferred {
    use super::*;

    /// Handled by both regions; the left one also moves the right region.
    pub struct Swap;
    impl Event for Swap {
        type Output = &'static str;
    }

    #[derive(Debug)]
    pub struct Duo;
    impl State for Duo {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Duo_Left>();
            def.region::<Duo_Right>().child::<Duo_Right2>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Duo_Left;
    impl State for Duo_Left {
        type Parent = Duo;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Swap>();
        }
    }
    impl Trigger<Swap> for Duo_Left {
        fn handle(&mut self, _: &Swap, ctx: &mut Context<Self>) -> &'static str {
            ctx.transition(Duo_Right2);
            "Duo_Left"
        }
    }

    #[derive(Debug, Default)]
    pub struct Duo_Right;
    impl State for Duo_Right {
        type Parent = Duo;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Swap>();
        }
    }
    impl Trigger<Swap> for Duo_Right {
        fn handle(&mut self, _: &Swap, _: &mut Context<Self>) -> &'static str {
            "Duo_Right"
        }
    }

    #[derive(Debug, Default)]
    pub struct Duo_Right2;
    impl State for Duo_Right2 {
        type Parent = Duo;
    }
}

#[test]
fn handler_transitions_apply_after_the_whole_dispatch() {
    use deferred::*;
    let mut chart = Statechart::new(Duo);
    assert_eq!(chart.trigger_all(Swap), Ok(vec!["Duo_Left", "Duo_Right"]));
    assert_eq!(chart.get_debug_name(), "Duo -> [Duo_Left, Duo_Right2]");
    assert_eq!(chart.trigger_all(Swap), Ok(vec!["Duo_Left"]));
}

#[test]
fn trigger_runs_every_eligible_handler_and_returns_the_first_output() {
    let mut chart = new_player();
    chart.trigger(Walk).unwrap();
    // Region 0 is handled by Player_Walking; region 1 bubbles up to Player, whose handler
    // also runs although `trigger` only returns the first output.
    assert_eq!(chart.trigger(Tick), Ok("Player_Walking"));
    assert_eq!(chart.root().ticks_handled, 1);
}

mod post_order_requests {
    use super::*;

    // Pair
    // ├── region 0: Pair_Left (initial)               handles Go -> Pair_Right2
    // └── region 1: Pair_Right (initial), Pair_Right2, Pair_Right3
    // Pair handles Go (on behalf of region 1) -> Pair_Right3

    pub struct Go;
    impl Event for Go {
        type Output = &'static str;
    }

    #[derive(Debug)]
    pub struct Pair;
    impl State for Pair {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Pair_Left>();
            def.region::<Pair_Right>()
                .child::<Pair_Right2>()
                .child::<Pair_Right3>();
            def.trigger::<Go>();
        }
    }
    impl Trigger<Go> for Pair {
        fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) -> &'static str {
            ctx.transition(Pair_Right3);
            "Pair"
        }
    }

    #[derive(Debug, Default)]
    pub struct Pair_Left;
    impl State for Pair_Left {
        type Parent = Pair;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Go>();
        }
    }
    impl Trigger<Go> for Pair_Left {
        fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) -> &'static str {
            ctx.transition(Pair_Right2);
            "Pair_Left"
        }
    }

    #[derive(Debug, Default)]
    pub struct Pair_Right;
    impl State for Pair_Right {
        type Parent = Pair;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Pair_Right2;
    impl State for Pair_Right2 {
        type Parent = Pair;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Pair_Right3;
    impl State for Pair_Right3 {
        type Parent = Pair;
        logging_hooks!();
    }
}

#[test]
fn an_ancestors_handler_request_applies_after_its_descendants_requests() {
    use post_order_requests::*;
    let mut chart = Statechart::new(Pair);
    common::take_log();
    // Pair's handler runs after Pair_Left's, so its request is applied last.
    assert_eq!(chart.trigger_all(Go), Ok(vec!["Pair_Left", "Pair"]));
    assert_eq!(
        take_log_without_updates(),
        [
            "exit Pair_Right",
            "enter Pair_Right2",
            "exit Pair_Right2",
            "enter Pair_Right3"
        ]
    );
    assert_eq!(chart.get_debug_name(), "Pair -> [Pair_Left, Pair_Right3]");
}

mod desk {
    use super::*;

    // Desk
    // ├── region 0: Desk_Lamp (initial)
    // └── region 1: Desk_Drawer (initial)
    //     Desk_Drawer
    //     └── region: Desk_Drawer_Shut (initial), Desk_Drawer_Open

    /// `Desk_Drawer_Shut` -> `Desk_Drawer_Open`.
    pub struct Open;
    impl Event for Open {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Desk;
    impl State for Desk {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Desk_Lamp>();
            def.region::<Desk_Drawer>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Desk_Lamp;
    impl State for Desk_Lamp {
        type Parent = Desk;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Desk_Drawer;
    impl State for Desk_Drawer {
        type Parent = Desk;
        fn define(def: &mut Definition<Self>) {
            def.region::<Desk_Drawer_Shut>().child::<Desk_Drawer_Open>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct Desk_Drawer_Shut;
    impl State for Desk_Drawer_Shut {
        type Parent = Desk_Drawer;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Open>();
        }
        logging_hooks!();
    }
    impl Trigger<Open> for Desk_Drawer_Shut {
        fn handle(&mut self, _: &Open, ctx: &mut Context<Self>) {
            ctx.transition(Desk_Drawer_Open);
        }
    }

    #[derive(Debug, Default)]
    pub struct Desk_Drawer_Open;
    impl State for Desk_Drawer_Open {
        type Parent = Desk_Drawer;
        logging_hooks!();
    }
}

#[test]
fn a_transition_below_an_active_state_of_a_later_region_keeps_that_state() {
    use desk::*;
    let mut chart = Statechart::new(Desk);
    common::take_log();
    chart.trigger(Open).unwrap();
    // Desk_Drawer (region 1) is the LCA: it is neither exited nor entered again.
    assert_eq!(
        take_log_without_updates(),
        ["exit Desk_Drawer_Shut", "enter Desk_Drawer_Open"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Desk -> [Desk_Lamp, Desk_Drawer -> Desk_Drawer_Open]"
    );
}

mod deep_cross_region {
    use super::*;

    // R
    // └── region: R_P (initial)
    //     R_P
    //     ├── region 0: R_P_L (initial)                handles Deep -> R_P_Q2_N2
    //     └── region 1: R_P_Q (initial), R_P_Q2
    //         R_P_Q2
    //         └── region: R_P_Q2_N (initial), R_P_Q2_N2

    /// Handled by `R_P_L`: a transition into the other region, two levels down.
    pub struct Deep;
    impl Event for Deep {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct R;
    impl State for R {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_P>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_P;
    impl State for R_P {
        type Parent = R;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_P_L>();
            def.region::<R_P_Q>().child::<R_P_Q2>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_P_L {
        pub handled: u32,
    }
    impl State for R_P_L {
        type Parent = R_P;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Deep>();
        }
        logging_hooks!();
    }
    impl Trigger<Deep> for R_P_L {
        fn handle(&mut self, _: &Deep, ctx: &mut Context<Self>) {
            self.handled += 1;
            ctx.transition(R_P_Q2_N2);
        }
    }

    #[derive(Debug, Default)]
    pub struct R_P_Q;
    impl State for R_P_Q {
        type Parent = R_P;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_P_Q2;
    impl State for R_P_Q2 {
        type Parent = R_P;
        fn define(def: &mut Definition<Self>) {
            def.region::<R_P_Q2_N>().child::<R_P_Q2_N2>();
        }
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_P_Q2_N;
    impl State for R_P_Q2_N {
        type Parent = R_P_Q2;
        logging_hooks!();
    }

    #[derive(Debug, Default)]
    pub struct R_P_Q2_N2;
    impl State for R_P_Q2_N2 {
        type Parent = R_P_Q2;
        logging_hooks!();
    }
}

/// User decision D4 for a target two levels below the LCA: the target is in region 1 of the
/// active, non-root parallel state `R_P`, below the inactive `R_P_Q2`. `R_P` is the LCA, so it
/// and its region 0 (where the requester `R_P_L` lives) are neither exited nor entered again.
#[test]
fn a_cross_region_transition_through_an_inactive_state_keeps_the_sibling_region() {
    use deep_cross_region::*;
    let mut chart = Statechart::new(R);
    common::take_log();
    chart.trigger(Deep).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit R_P_Q", "enter R_P_Q2", "enter R_P_Q2_N2"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "R -> R_P -> [R_P_L { handled: 1 }, R_P_Q2 -> R_P_Q2_N2]"
    );
    // The requester is the same instance, so its later requests still apply.
    chart.trigger(Deep).unwrap();
    assert_eq!(
        take_log_without_updates(),
        ["exit R_P_Q2_N2", "enter R_P_Q2_N2"]
    );
    assert_eq!(chart.state::<R_P_L>().unwrap().handled, 2);
}
