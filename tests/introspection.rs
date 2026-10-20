//! Spec item 13: the Debug format of the active configuration, `get_recursive_active_type_ids`,
//! `is_active` and `state`.
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

    // A -> A_B { n } -> A_B_C, with A_X as an inactive sibling of A_B.

    #[derive(Debug)]
    pub struct A;
    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_B>().child::<A_X>();
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
