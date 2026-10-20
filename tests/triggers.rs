//! Trigger dispatch on a single active chain: deepest handler wins, fallback walks every
//! ancestor up to the root, typed outputs, payloads, and deferred transitions.
#![allow(non_camel_case_types)]

use statechart::prelude::*;

// Root { calls }
// └── region: Root_Mid (initial), Root_Other
//     Root_Mid
//     └── region: Root_Mid_Leaf (initial)

/// Handled by `Root_Mid_Leaf` and by `Root`.
struct Ping;
impl Event for Ping {
    type Output = &'static str;
}

/// Handled by `Root` only: the leaf and its parent do not handle it.
struct Deep;
impl Event for Deep {
    type Output = &'static str;
}

/// Handled by nobody.
struct Nobody;
impl Event for Nobody {
    type Output = ();
}

/// Handled by `Root_Other` only, which is not active at first.
struct OtherOnly;
impl Event for OtherOnly {
    type Output = ();
}

/// Carries a payload: adds `.0` to the leaf's total and returns the new total.
struct Add(u32);
impl Event for Add {
    type Output = u32;
}

/// The leaf requests `Root_Other` and returns whether it is itself still active.
struct Leave;
impl Event for Leave {
    type Output = bool;
}

/// The leaf implements `Trigger<Forgotten>` but never registers it with `def.trigger`.
struct Forgotten;
impl Event for Forgotten {
    type Output = ();
}

#[derive(Debug, Default)]
struct Root {
    calls: u32,
}
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Mid>().child::<Root_Other>();
        def.trigger::<Ping>().trigger::<Deep>();
    }
}
impl Trigger<Ping> for Root {
    fn handle(&mut self, _: &Ping, _: &mut Context<Self>) -> &'static str {
        self.calls += 1;
        "Root"
    }
}
impl Trigger<Deep> for Root {
    fn handle(&mut self, _: &Deep, _: &mut Context<Self>) -> &'static str {
        self.calls += 1;
        "Root"
    }
}

#[derive(Debug, Default)]
struct Root_Mid;
impl State for Root_Mid {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Mid_Leaf>();
    }
}

#[derive(Debug, Default)]
struct Root_Mid_Leaf {
    total: u32,
}
impl State for Root_Mid_Leaf {
    type Parent = Root_Mid;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Ping>().trigger::<Add>().trigger::<Leave>();
    }
}
impl Trigger<Ping> for Root_Mid_Leaf {
    fn handle(&mut self, _: &Ping, _: &mut Context<Self>) -> &'static str {
        "Root_Mid_Leaf"
    }
}
impl Trigger<Add> for Root_Mid_Leaf {
    fn handle(&mut self, add: &Add, _: &mut Context<Self>) -> u32 {
        self.total += add.0;
        self.total
    }
}
impl Trigger<Leave> for Root_Mid_Leaf {
    fn handle(&mut self, _: &Leave, ctx: &mut Context<Self>) -> bool {
        ctx.transition(Root_Other);
        ctx.is_active::<Root_Mid_Leaf>()
    }
}
// Deliberately NOT registered in `Root_Mid_Leaf::define`.
impl Trigger<Forgotten> for Root_Mid_Leaf {
    fn handle(&mut self, _: &Forgotten, _: &mut Context<Self>) {
        panic!("an unregistered handler must never run");
    }
}

#[derive(Debug, Default)]
struct Root_Other;
impl State for Root_Other {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<OtherOnly>();
    }
}
impl Trigger<OtherOnly> for Root_Other {
    fn handle(&mut self, _: &OtherOnly, _: &mut Context<Self>) {}
}

#[test]
fn the_deepest_handler_wins_and_only_it_runs() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Ping), Ok("Root_Mid_Leaf"));
    assert_eq!(chart.root().calls, 0);
}

#[test]
fn fallback_skips_states_without_a_handler_up_to_the_root() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Deep), Ok("Root"));
    assert_eq!(chart.trigger_all(Deep), Ok(vec!["Root"]));
    assert_eq!(chart.root().calls, 2);
}

#[test]
fn a_trigger_nobody_in_the_active_chain_handles_is_not_handled() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Nobody), Err(NotHandled));
    assert_eq!(chart.trigger_all(Nobody), Err(NotHandled));
}

#[test]
fn inactive_states_do_not_handle_triggers() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(OtherOnly), Err(NotHandled));
    chart.trigger(Leave).unwrap();
    assert_eq!(chart.trigger(OtherOnly), Ok(()));
}

#[test]
fn the_handler_reads_the_trigger_payload_and_returns_a_typed_output() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Add(5)), Ok(5));
    assert_eq!(chart.trigger(Add(2)), Ok(7));
    assert_eq!(chart.state::<Root_Mid_Leaf>().unwrap().total, 7);
}

#[test]
fn transitions_requested_by_a_handler_apply_after_the_dispatch() {
    let mut chart = Statechart::new(Root::default());
    // Still active while the handler runs; replaced right after.
    assert_eq!(chart.trigger(Leave), Ok(true));
    assert_eq!(chart.get_debug_name(), "Root { calls: 0 } -> Root_Other");
}

/// User decision D2: a handler only runs if its state registers the trigger with
/// `def.trigger::<E>()` in `define`. Implementing `Trigger<E>` alone is silently ignored.
#[test]
fn an_implemented_but_unregistered_handler_never_runs_and_is_not_handled() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Forgotten), Err(NotHandled));
    assert_eq!(chart.trigger_all(Forgotten), Err(NotHandled));
}

#[test]
fn not_handled_is_an_error_with_a_message() {
    let error: Box<dyn std::error::Error> = Box::new(NotHandled);
    assert_eq!(error.to_string(), "no active state handles this trigger");
}
