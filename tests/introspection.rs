//! Spec item 13: the Debug format of the active configuration, `get_recursive_active_type_ids`,
//! `is_active`, `state` and `state_mut`; and `Debug` for the contexts and the definition.
#![allow(non_camel_case_types)]

use statechart::prelude::*;
use std::any::TypeId;

mod lone {
    use super::*;

    #[derive(Debug)]
    pub struct Lone;
    impl State for Lone {
        type Parent = NoParent;
    }
}

#[test]
fn a_lone_root_prints_as_itself_and_has_no_active_descendants() {
    let chart = Statechart::new(lone::Lone);
    assert_eq!(chart.get_debug_name(), "Lone");
    assert!(chart.get_recursive_active_type_ids().is_empty());
    assert!(chart.is_active::<lone::Lone>());
}

mod chain {
    use super::*;

    // A -> A_B { n } -> A_B_C, with the leaf A_X as an inactive sibling of A_B. A handles `GoX`
    // (-> A_X).

    pub struct GoX;
    impl Event for GoX {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct A;
    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_B>().child::<A_X>();
            def.trigger::<GoX>();
        }
    }
    impl Trigger<GoX> for A {
        fn handle(&mut self, _: &GoX, ctx: &mut Context<Self>) {
            ctx.transition(A_X);
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B {
        pub n: u32,
    }
    impl State for A_B {
        type Parent = A;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_B_C>();
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B_C;
    impl State for A_B_C {
        type Parent = A_B;
    }

    #[derive(Debug, Default)]
    pub struct A_X;
    impl State for A_X {
        type Parent = A;
    }
}

#[test]
fn a_single_chain_prints_with_arrows_and_each_states_own_debug() {
    let chart = Statechart::new(chain::A);
    assert_eq!(chart.get_debug_name(), "A -> A_B { n: 0 } -> A_B_C");
    assert_eq!(format!("{chart:?}"), chart.get_debug_name());
}

#[test]
fn active_type_ids_of_a_chain_are_deepest_first_without_the_root() {
    let chart = Statechart::new(chain::A);
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        vec![TypeId::of::<chain::A_B_C>(), TypeId::of::<chain::A_B>()]
    );
}

#[test]
fn is_active_and_state_follow_the_active_configuration() {
    use chain::*;
    let chart = Statechart::new(A);
    assert!(chart.is_active::<A>());
    assert!(chart.is_active::<A_B>());
    assert!(chart.is_active::<A_B_C>());
    assert!(!chart.is_active::<A_X>());
    assert_eq!(chart.state::<A_B>().unwrap().n, 0);
    assert!(chart.state::<A_X>().is_none());
    assert!(chart.state::<A>().is_some());
}

/// `state` / `state_mut` follow the static path of `T` through the active tree. When an
/// ancestor of `T` was replaced by a leaf, the walk stops there: `None`, not a panic.
#[test]
fn state_of_a_deep_state_whose_ancestor_was_replaced_by_a_leaf_is_none() {
    use chain::*;
    let mut chart = Statechart::new(A);
    assert!(chart.state::<A_B_C>().is_some());
    chart.trigger(GoX).unwrap();
    assert_eq!(chart.get_debug_name(), "A -> A_X");
    assert!(chart.state::<A_B_C>().is_none());
    assert!(chart.state_mut::<A_B_C>().is_none());
    assert!(!chart.is_active::<A_B_C>());
}

mod robot {
    use super::*;

    // Robot -> Robot_Body -> [Robot_Body_Legs -> Robot_Body_Legs_Still, Robot_Body_Arms]

    #[derive(Debug)]
    pub struct Robot;
    impl State for Robot {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Robot_Body>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Robot_Body;
    impl State for Robot_Body {
        type Parent = Robot;
        fn define(def: &mut Definition<Self>) {
            def.region::<Robot_Body_Legs>();
            def.region::<Robot_Body_Arms>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Robot_Body_Legs;
    impl State for Robot_Body_Legs {
        type Parent = Robot_Body;
        fn define(def: &mut Definition<Self>) {
            def.region::<Robot_Body_Legs_Still>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Robot_Body_Legs_Still;
    impl State for Robot_Body_Legs_Still {
        type Parent = Robot_Body_Legs;
    }

    #[derive(Debug, Default)]
    pub struct Robot_Body_Arms;
    impl State for Robot_Body_Arms {
        type Parent = Robot_Body;
    }
}

#[test]
fn parallel_regions_print_in_brackets_separated_by_commas() {
    let chart = Statechart::new(robot::Robot);
    assert_eq!(
        chart.get_debug_name(),
        "Robot -> Robot_Body -> [Robot_Body_Legs -> Robot_Body_Legs_Still, Robot_Body_Arms]"
    );
    assert_eq!(format!("{chart:?}"), chart.get_debug_name());
}

#[test]
fn active_type_ids_with_parallel_regions_are_post_order() {
    use robot::*;
    let chart = Statechart::new(Robot);
    // Deepest first within a branch, branches in region order, root excluded.
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        vec![
            TypeId::of::<Robot_Body_Legs_Still>(),
            TypeId::of::<Robot_Body_Legs>(),
            TypeId::of::<Robot_Body_Arms>(),
            TypeId::of::<Robot_Body>(),
        ]
    );
}

#[test]
fn state_finds_active_states_in_every_region() {
    use robot::*;
    let chart = Statechart::new(Robot);
    assert!(chart.state::<Robot_Body>().is_some());
    assert!(chart.state::<Robot_Body_Legs_Still>().is_some());
    // Region 1 of Robot_Body.
    assert!(chart.state::<Robot_Body_Arms>().is_some());
}

#[test]
fn state_mut_changes_the_data_of_an_active_state() {
    use chain::*;
    let mut chart = Statechart::new(A);
    chart.state_mut::<A_B>().unwrap().n = 5;
    assert_eq!(chart.state::<A_B>().unwrap().n, 5);
    assert_eq!(chart.get_debug_name(), "A -> A_B { n: 5 } -> A_B_C");
    assert!(chart.state_mut::<A_X>().is_none());
    assert!(chart.state_mut::<A>().is_some());

    let mut robot = Statechart::new(robot::Robot);
    assert!(robot.state_mut::<robot::Robot_Body_Arms>().is_some());
}

mod uneven {
    use super::*;

    // R -> [R_A, R_B -> R_B_1]: the earlier region is shallower than the later one.

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
fn active_type_ids_are_post_order_not_sorted_by_depth() {
    use uneven::*;
    let chart = Statechart::new(R);
    // R_A (depth 1) comes before R_B_1 (depth 2): deepest first only within a branch.
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        vec![
            TypeId::of::<R_A>(),
            TypeId::of::<R_B_1>(),
            TypeId::of::<R_B>()
        ]
    );
}

/// `Context`, `ExitContext`, `Definition` and `Region` implement `Debug` (Rust API guideline
/// C-DEBUG), so a hook or `define` can `dbg!` what it was given.
mod debug_impls {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub fn seen(entry: String) {
        SEEN.with(|seen| seen.borrow_mut().push(entry));
    }

    pub fn take_seen() -> Vec<String> {
        SEEN.with(|seen| std::mem::take(&mut *seen.borrow_mut()))
    }

    // Root { n } -> Root_A (initial) | Root_B. Root handles `GoB` (-> Root_B).

    pub struct GoB;
    impl Event for GoB {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root {
        pub n: u32,
    }
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            seen(format!("{def:?}"));
            let region = def.region::<Root_A>();
            seen(format!("{region:?}"));
            let region = region.child::<Root_B>();
            seen(format!("{region:?}"));
            def.trigger::<GoB>();
            seen(format!("{def:?}"));
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            seen(format!("{ctx:?}"));
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
        fn update(&mut self, ctx: &mut Context<Self>) {
            seen(format!("{ctx:?}"));
        }
        fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
            seen(format!("{ctx:?}"));
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
    }
}

#[test]
fn the_contexts_and_the_definition_implement_debug() {
    use debug_impls::*;
    let mut chart = Statechart::new(Root { n: 1 });
    assert_eq!(
        take_seen(),
        [
            r#"Definition { state: "introspection::debug_impls::Root", regions: 0, triggers: 0 }"#,
            r#"Region { state: "introspection::debug_impls::Root", index: 0, children: 1 }"#,
            r#"Region { state: "introspection::debug_impls::Root", index: 0, children: 2 }"#,
            r#"Definition { state: "introspection::debug_impls::Root", regions: 1, triggers: 1 }"#,
        ]
    );
    chart.update();
    chart.trigger(GoB).unwrap();
    assert_eq!(
        take_seen(),
        [
            r#"Context { state: "introspection::debug_impls::Root", parent: None, .. }"#,
            r#"Context { state: "introspection::debug_impls::Root_A", parent: Some(Root { n: 1 }), .. }"#,
            r#"ExitContext { state: "introspection::debug_impls::Root_A", parent: Some(Root { n: 1 }), .. }"#,
        ]
    );
    assert_eq!(chart.root().n, 1);
}
