//! Batch semantics (spec item 10): transitions are applied after the traversal, in request
//! order; a request made by a state that an earlier transition in the same batch exited is
//! dropped (decided per instance, not per type); requests made in `on_enter` join the end of
//! the batch.
#![allow(non_camel_case_types)]

#[macro_use]
mod common;

use common::{log, take_log, take_log_without_updates};
use statechart::prelude::*;

// Root { plan }
// ├── region 0: Root_Left (initial), Root_Left2
// │   Root_Left
// │   └── region: Root_Left_Inner (initial), Root_Left_Inner2
// └── region 1: Root_Right (initial), Root_Right2, Root_Right3
//
// `Root.plan` decides what each state requests. Update order (pre-order):
// Root, Root_Left, Root_Left_Inner, Root_Right.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Plan {
    /// Root -> Root_Left2, then Root_Left_Inner -> Root_Left_Inner2 (dropped: exited).
    ParentThenExitedChild,
    /// Root -> Root_Left (re-entered), then Root_Left_Inner -> Root_Right2 (dropped: the
    /// requesting instance was exited, even though a new Root_Left_Inner is active).
    ReenteredRequester,
    /// Root_Left_Inner -> Root_Left_Inner2, then Root_Right -> Root_Right2 (both apply).
    IndependentRegions,
    /// Root_Left_Inner -> Root_Left2, then Root_Right -> Root_Left_Inner2 (applied in order:
    /// the second one wins).
    SameRegionInOrder,
    /// Root_Left_Inner -> Root_Left2 (whose on_enter requests Root_Right3), then
    /// Root_Right -> Root_Right2. The on_enter request runs after Root_Right2.
    OnEnterJoinsTheEnd,
    /// Root_Left_Inner -> Root_Left2 (whose on_enter requests Root_Right3), then
    /// Root_Right -> Root_Left, which exits Root_Left2: its on_enter request is dropped.
    OnEnterRequesterExited,
}

#[derive(Debug)]
struct Root {
    plan: Plan,
}
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Left>().child::<Root_Left2>();
        def.region::<Root_Right>()
            .child::<Root_Right2>()
            .child::<Root_Right3>();
    }
    fn update(&mut self, ctx: &mut Context<Self>) {
        log("update", "Root");
        match self.plan {
            Plan::ParentThenExitedChild => ctx.transition(Root_Left2),
            Plan::ReenteredRequester => ctx.transition(Root_Left),
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
struct Root_Left;
impl State for Root_Left {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Left_Inner>().child::<Root_Left_Inner2>();
    }
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_Left_Inner;
impl State for Root_Left_Inner {
    type Parent = Root_Left;
    fn on_enter(&mut self, _: &mut Context<Self>) {
        log("enter", "Root_Left_Inner");
    }
    fn on_exit(&mut self, _: &mut ExitContext<Self>) {
        log("exit", "Root_Left_Inner");
    }
    fn update(&mut self, ctx: &mut Context<Self>) {
        log("update", "Root_Left_Inner");
        match ctx.ancestor::<Root>().unwrap().plan {
            Plan::ParentThenExitedChild | Plan::IndependentRegions => {
                ctx.transition(Root_Left_Inner2)
            }
            Plan::ReenteredRequester => ctx.transition(Root_Right2),
            Plan::SameRegionInOrder | Plan::OnEnterJoinsTheEnd | Plan::OnEnterRequesterExited => {
                ctx.transition(Root_Left2)
            }
        }
    }
}

#[derive(Debug, Default)]
struct Root_Left_Inner2;
impl State for Root_Left_Inner2 {
    type Parent = Root_Left;
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_Left2;
impl State for Root_Left2 {
    type Parent = Root;
    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        log("enter", "Root_Left2");
        if matches!(
            ctx.parent().plan,
            Plan::OnEnterJoinsTheEnd | Plan::OnEnterRequesterExited
        ) {
            ctx.transition(Root_Right3);
        }
    }
    fn on_exit(&mut self, _: &mut ExitContext<Self>) {
        log("exit", "Root_Left2");
    }
}

#[derive(Debug, Default)]
struct Root_Right;
impl State for Root_Right {
    type Parent = Root;
    fn on_enter(&mut self, _: &mut Context<Self>) {
        log("enter", "Root_Right");
    }
    fn on_exit(&mut self, _: &mut ExitContext<Self>) {
        log("exit", "Root_Right");
    }
    fn update(&mut self, ctx: &mut Context<Self>) {
        log("update", "Root_Right");
        match ctx.parent().plan {
            Plan::IndependentRegions | Plan::OnEnterJoinsTheEnd => ctx.transition(Root_Right2),
            Plan::SameRegionInOrder => ctx.transition(Root_Left_Inner2),
            Plan::OnEnterRequesterExited => ctx.transition(Root_Left),
            Plan::ParentThenExitedChild | Plan::ReenteredRequester => {}
        }
    }
}

#[derive(Debug, Default)]
struct Root_Right2;
impl State for Root_Right2 {
    type Parent = Root;
    logging_hooks!();
}

#[derive(Debug, Default)]
struct Root_Right3;
impl State for Root_Right3 {
    type Parent = Root;
    logging_hooks!();
}

fn run(plan: Plan) -> (Statechart<Root>, Vec<String>) {
    let mut chart = Statechart::new(Root { plan });
    take_log();
    chart.update();
    let log = take_log();
    (chart, log)
}

#[test]
fn requests_are_applied_only_after_the_whole_traversal() {
    let (_, log) = run(Plan::ParentThenExitedChild);
    // Root asked for Root_Left2 first, yet Root_Left, Root_Left_Inner and Root_Right still
    // ran their update before anything was exited.
    assert_eq!(
        log,
        [
            "update Root",
            "update Root_Left",
            "update Root_Left_Inner",
            "update Root_Right",
            "exit Root_Left_Inner",
            "exit Root_Left",
            "enter Root_Left2",
        ]
    );
}

#[test]
fn a_request_from_a_state_exited_earlier_in_the_batch_is_dropped() {
    let (chart, _) = run(Plan::ParentThenExitedChild);
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: ParentThenExitedChild } -> [Root_Left2, Root_Right]"
    );
}

#[test]
fn a_request_from_an_exited_instance_is_dropped_even_if_its_type_was_re_entered() {
    let (chart, log) = run(Plan::ReenteredRequester);
    assert_eq!(
        log.into_iter()
            .filter(|l| !l.starts_with("update"))
            .collect::<Vec<_>>(),
        [
            "exit Root_Left_Inner",
            "exit Root_Left",
            "enter Root_Left",
            "enter Root_Left_Inner"
        ]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: ReenteredRequester } -> [Root_Left -> Root_Left_Inner, Root_Right]"
    );
}

#[test]
fn requests_in_independent_regions_all_apply() {
    let (chart, _) = run(Plan::IndependentRegions);
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: IndependentRegions } -> [Root_Left -> Root_Left_Inner2, Root_Right2]"
    );
}

#[test]
fn conflicting_requests_apply_in_request_order() {
    let (chart, _) = run(Plan::SameRegionInOrder);
    // Root_Left_Inner asked for Root_Left2 first; Root_Right (not exited) then asked for
    // Root_Left_Inner2, which re-enters Root_Left and replaces Root_Left2.
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: SameRegionInOrder } -> [Root_Left -> Root_Left_Inner2, Root_Right]"
    );
}

#[test]
fn on_enter_requests_join_the_end_of_the_batch() {
    let (chart, log) = run(Plan::OnEnterJoinsTheEnd);
    assert_eq!(
        log.into_iter()
            .filter(|l| l.starts_with("enter"))
            .collect::<Vec<_>>(),
        ["enter Root_Left2", "enter Root_Right2", "enter Root_Right3"]
    );
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: OnEnterJoinsTheEnd } -> [Root_Left2, Root_Right3]"
    );
}

#[test]
fn an_on_enter_request_is_dropped_when_its_requester_is_exited_first() {
    let (chart, _) = run(Plan::OnEnterRequesterExited);
    assert_eq!(
        chart.get_debug_name(),
        "Root { plan: OnEnterRequesterExited } -> [Root_Left -> Root_Left_Inner, Root_Right]"
    );
}

mod trio {
    use super::*;

    // Trio
    // ├── region 0: Trio_A (initial)
    // ├── region 1: Trio_B (initial)
    // └── region 2: Trio_C (initial), Trio_C1, Trio_C2
    //
    // Both Trio_A and Trio_B handle `Go`; each requests a different state of region 2.

    pub struct Go;
    impl Event for Go {
        type Output = &'static str;
    }

    #[derive(Debug)]
    pub struct Trio;
    impl State for Trio {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Trio_A>();
            def.region::<Trio_B>();
            def.region::<Trio_C>().child::<Trio_C1>().child::<Trio_C2>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Trio_A;
    impl State for Trio_A {
        type Parent = Trio;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Go>();
        }
    }
    impl Trigger<Go> for Trio_A {
        fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) -> &'static str {
            ctx.transition(Trio_C1);
            "Trio_A"
        }
    }

    #[derive(Debug, Default)]
    pub struct Trio_B;
    impl State for Trio_B {
        type Parent = Trio;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Go>();
        }
    }
    impl Trigger<Go> for Trio_B {
        fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) -> &'static str {
            ctx.transition(Trio_C2);
            "Trio_B"
        }
    }

    #[derive(Debug, Default)]
    pub struct Trio_C;
    impl State for Trio_C {
        type Parent = Trio;
    }

    #[derive(Debug, Default)]
    pub struct Trio_C1;
    impl State for Trio_C1 {
        type Parent = Trio;
    }

    #[derive(Debug, Default)]
    pub struct Trio_C2;
    impl State for Trio_C2 {
        type Parent = Trio;
    }
}

#[test]
fn handler_requests_apply_in_the_order_the_handlers_ran() {
    use trio::*;
    let mut chart = Statechart::new(Trio);
    assert_eq!(chart.trigger_all(Go), Ok(vec!["Trio_A", "Trio_B"]));
    assert_eq!(chart.get_debug_name(), "Trio -> [Trio_A, Trio_B, Trio_C2]");
}

mod transient {
    use super::*;

    // Root -> Root_Boot (initial) whose on_enter immediately moves on to Root_Ready.

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_Boot>().child::<Root_Ready>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_Boot;
    impl State for Root_Boot {
        type Parent = Root;
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            log("enter", "Root_Boot");
            ctx.transition(Root_Ready);
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            log("exit", "Root_Boot");
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_Ready;
    impl State for Root_Ready {
        type Parent = Root;
        logging_hooks!();
    }
}

#[test]
fn on_enter_requests_made_while_building_the_chart_apply_before_new_returns() {
    let chart = Statechart::new(transient::Root);
    assert_eq!(
        take_log_without_updates(),
        ["enter Root_Boot", "exit Root_Boot", "enter Root_Ready"]
    );
    assert_eq!(chart.get_debug_name(), "Root -> Root_Ready");
}
