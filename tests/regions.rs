//! Parallel regions: initial states, independence, per-region trigger dispatch, a shared
//! ancestor running at most once, cross-region transitions and "in state" guards.
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
            .trigger::<WalkFromWeapon>();
    }
    logging_hooks!();
}
impl Trigger<Ping> for Player_Holstered {
    fn handle(&mut self, _: &Ping, _: &mut Context<Self>) -> &'static str {
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
