//! End-to-end edge cases on one chart with three regions and nested parallel regions (the
//! adversarial cases the design reviewers checked).
//!
//! ```text
//! R { foo, exits }
//! ├── region 0: R_A (initial)
//! │   R_A
//! │   └── region: R_A_X (initial), R_A_W
//! ├── region 1: R_B (initial)
//! │   R_B
//! │   └── region: R_B_Y (initial), R_B_Z
//! └── region 2: R_C (initial), R_Q
//!     R_Q
//!     ├── region 0: R_Q_L (initial)
//!     └── region 1: R_Q_R (initial)
//!         R_Q_R
//!         └── region: R_Q_R_1 (initial), R_Q_R_2
//! ```
//!
//! `Foo` is handled by `R`, `R_A_X`, `R_Q` and `R_Q_L`. A per-thread plan says which hook of
//! which state requests which transition.
#![allow(non_camel_case_types)]
use statechart::prelude::*;
use std::cell::RefCell;
use std::fmt;

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static PLAN: RefCell<Vec<(&'static str, &'static str)>> = const { RefCell::new(Vec::new()) };
}
fn log(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}
fn take() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}
fn plan(who: &'static str, what: &'static str) {
    PLAN.with(|p| p.borrow_mut().push((who, what)));
}
fn planned(who: &str) -> Option<&'static str> {
    PLAN.with(|p| p.borrow().iter().find(|(w, _)| *w == who).map(|(_, x)| *x))
}
fn clear_plan() {
    PLAN.with(|p| p.borrow_mut().clear());
}
fn name<S>() -> &'static str {
    std::any::type_name::<S>().rsplit("::").next().unwrap()
}

struct Foo;
impl Event for Foo {
    type Output = &'static str;
}

macro_rules! go {
    ($ctx:expr, $what:expr) => {
        match $what {
            "R" => $ctx.transition(R { foo: 0, exits: 0 }),
            "R_A" => $ctx.transition(R_A),
            "R_A_W" => $ctx.transition(R_A_W),
            "R_B_Z" => $ctx.transition(R_B_Z),
            "R_Q_R_2" => $ctx.transition(R_Q_R_2),
            "R_C" => $ctx.transition(R_C),
            w => panic!("{w}"),
        }
    };
}

macro_rules! hooks {
    () => {
        fn on_enter(&mut self, ctx: &mut Context<Self>) {
            log(format!("enter {}", name::<Self>()));
            if let Some(w) = planned(&format!("enter {}", name::<Self>())) {
                go!(ctx, w);
            }
        }
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            log(format!("exit {}", name::<Self>()));
            ctx.ancestor_mut::<R>().unwrap().exits += 1;
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            log(format!("update {}", name::<Self>()));
            if let Some(w) = planned(&format!("update {}", name::<Self>())) {
                go!(ctx, w);
            }
        }
    };
}

struct R {
    foo: u32,
    exits: u32,
}
impl fmt::Debug for R {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("R")
    }
}
impl State for R {
    type Parent = NoParent;
    fn define(d: &mut Definition<Self>) {
        d.region::<R_A>();
        d.region::<R_B>();
        d.region::<R_C>().child::<R_Q>();
        d.trigger::<Foo>();
    }
    fn on_enter(&mut self, _ctx: &mut Context<Self>) {
        log("enter R".into());
    }
    fn on_exit(&mut self, _ctx: &mut ExitContext<Self>) {
        log("exit R".into());
    }
    fn update(&mut self, ctx: &mut Context<Self>) {
        log("update R".into());
        if let Some(w) = planned("update R") {
            go!(ctx, w);
        }
    }
}
impl Trigger<Foo> for R {
    fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
        self.foo += 1;
        "R"
    }
}

macro_rules! leaf {
    ($n:ident, $p:ty) => {
        #[derive(Debug, Default)]
        struct $n;
        impl State for $n {
            type Parent = $p;
            hooks!();
        }
    };
}

#[derive(Debug, Default)]
struct R_A;
impl State for R_A {
    type Parent = R;
    fn define(d: &mut Definition<Self>) {
        d.region::<R_A_X>().child::<R_A_W>();
    }
    hooks!();
}
#[derive(Debug, Default)]
struct R_A_X;
impl State for R_A_X {
    type Parent = R_A;
    fn define(d: &mut Definition<Self>) {
        d.trigger::<Foo>();
    }
    hooks!();
}
impl Trigger<Foo> for R_A_X {
    fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
        "R_A_X"
    }
}
leaf!(R_A_W, R_A);

#[derive(Debug, Default)]
struct R_B;
impl State for R_B {
    type Parent = R;
    fn define(d: &mut Definition<Self>) {
        d.region::<R_B_Y>().child::<R_B_Z>();
    }
    hooks!();
}
leaf!(R_B_Y, R_B);
leaf!(R_B_Z, R_B);
leaf!(R_C, R);

#[derive(Debug, Default)]
struct R_Q;
impl State for R_Q {
    type Parent = R;
    fn define(d: &mut Definition<Self>) {
        d.region::<R_Q_L>();
        d.region::<R_Q_R>();
        d.trigger::<Foo>();
    }
    hooks!();
}
impl Trigger<Foo> for R_Q {
    fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
        "R_Q"
    }
}
#[derive(Debug, Default)]
struct R_Q_L;
impl State for R_Q_L {
    type Parent = R_Q;
    fn define(d: &mut Definition<Self>) {
        d.trigger::<Foo>();
    }
    hooks!();
}
impl Trigger<Foo> for R_Q_L {
    fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
        "R_Q_L"
    }
}
#[derive(Debug, Default)]
struct R_Q_R;
impl State for R_Q_R {
    type Parent = R_Q;
    fn define(d: &mut Definition<Self>) {
        d.region::<R_Q_R_1>().child::<R_Q_R_2>();
    }
    hooks!();
}
leaf!(R_Q_R_1, R_Q_R);
leaf!(R_Q_R_2, R_Q_R);

fn start() -> Statechart<R> {
    clear_plan();
    let c = Statechart::new(R { foo: 0, exits: 0 });
    take();
    c
}

#[test]
fn construction_enters_every_region_in_declaration_order() {
    clear_plan();
    let c = Statechart::new(R { foo: 0, exits: 0 });
    assert_eq!(
        take(),
        [
            "enter R",
            "enter R_A",
            "enter R_A_X",
            "enter R_B",
            "enter R_B_Y",
            "enter R_C"
        ]
    );
    assert_eq!(c.get_debug_name(), "R -> [R_A -> R_A_X, R_B -> R_B_Y, R_C]");
}

#[test]
fn the_root_handles_a_trigger_once_for_every_unhandled_branch() {
    let mut c = start();
    assert_eq!(c.trigger_all(Foo), Ok(vec!["R_A_X", "R"]));
    assert_eq!(c.root().foo, 1);
}

#[test]
fn a_jump_into_an_inactive_parallel_subtree_enters_every_region_then_dispatch_is_post_order() {
    let mut c = start();
    plan("update R_C", "R_Q_R_2");
    c.update();
    let l = take();
    let tail: Vec<_> = l
        .iter()
        .filter(|s| !s.starts_with("update"))
        .cloned()
        .collect();
    assert_eq!(
        tail,
        [
            "exit R_C",
            "enter R_Q",
            "enter R_Q_L",
            "enter R_Q_R",
            "enter R_Q_R_2"
        ]
    );
    assert_eq!(
        c.get_debug_name(),
        "R -> [R_A -> R_A_X, R_B -> R_B_Y, R_Q -> [R_Q_L, R_Q_R -> R_Q_R_2]]"
    );
    let before = c.root().foo;
    assert_eq!(c.trigger_all(Foo), Ok(vec!["R_A_X", "R_Q_L", "R_Q", "R"]));
    assert_eq!(c.root().foo, before + 1);
}

#[test]
fn a_request_from_a_child_exited_by_its_parents_request_is_dropped() {
    let mut c = start();
    plan("update R_A", "R_A_W");
    plan("update R_A_X", "R_B_Z");
    c.update();
    assert!(c.is_active::<R_A_W>());
    assert!(c.is_active::<R_B_Y>(), "R_A_X's request must be dropped");
}

#[test]
fn a_request_from_a_re_entered_state_is_dropped() {
    let mut c = start();
    plan("update R", "R_A");
    plan("update R_A_X", "R_B_Z");
    c.update();
    assert!(c.is_active::<R_A_X>());
    assert!(
        c.is_active::<R_B_Y>(),
        "R_A_X was exited (and re-entered) so its old request must be dropped"
    );
}

#[test]
fn the_first_request_wins_when_it_exits_the_second_requester() {
    let mut c = start();
    plan("update R_A_X", "R_B_Z");
    plan("update R_B_Y", "R_A_W");
    c.update();
    assert!(c.is_active::<R_B_Z>());
    assert!(c.is_active::<R_A_X>(), "R_B_Y's request dropped");
}

#[test]
fn targeting_the_active_parent_re_enters_it_and_leaves_other_regions_alone() {
    let mut c = start();
    plan("update R_A_X", "R_A");
    c.update();
    let l: Vec<_> = take()
        .into_iter()
        .filter(|s| !s.starts_with("update"))
        .collect();
    assert_eq!(l, ["exit R_A_X", "exit R_A", "enter R_A", "enter R_A_X"]);
}

#[test]
fn restarting_the_root_exits_everything_then_enters_everything() {
    let mut c = start();
    plan("update R_B_Y", "R");
    c.update();
    let l: Vec<_> = take()
        .into_iter()
        .filter(|s| !s.starts_with("update"))
        .collect();
    assert_eq!(
        l,
        [
            "exit R_A_X",
            "exit R_A",
            "exit R_B_Y",
            "exit R_B",
            "exit R_C",
            "exit R",
            "enter R",
            "enter R_A",
            "enter R_A_X",
            "enter R_B",
            "enter R_B_Y",
            "enter R_C"
        ]
    );
}

#[test]
fn an_on_enter_request_during_construction_reaches_an_inactive_parallel_subtree() {
    clear_plan();
    plan("enter R_C", "R_Q_R_2");
    let c = Statechart::new(R { foo: 0, exits: 0 });
    assert!(c.is_active::<R_Q_R_2>());
    assert!(!c.is_active::<R_C>());
    clear_plan();
}

#[test]
fn an_on_enter_request_is_applied_in_the_same_batch() {
    let mut c = start();
    // R_Q_R_2 on_enter -> R_C; applied in same batch
    plan("update R_C", "R_Q_R_2");
    plan("enter R_Q_R_2", "R_C");
    c.update();
    assert!(c.is_active::<R_C>());
    assert!(!c.is_active::<R_Q>());
}

#[test]
fn active_type_ids_are_post_order_across_regions() {
    let c = start();
    use std::any::TypeId;
    assert_eq!(
        c.get_recursive_active_type_ids(),
        vec![
            TypeId::of::<R_A_X>(),
            TypeId::of::<R_A>(),
            TypeId::of::<R_B_Y>(),
            TypeId::of::<R_B>(),
            TypeId::of::<R_C>()
        ]
    );
}

#[test]
fn a_trigger_nobody_registered_is_not_handled() {
    struct Bar;
    impl Event for Bar {
        type Output = ();
    }
    let mut c = start();
    assert_eq!(c.trigger(Bar), Err(NotHandled));
}
