#![allow(non_camel_case_types)]

use std::any::TypeId;
use statechart::custom_state_trait::CustomStateTrait;
use statechart::state::State;
use statechart::statechart::Statechart;
use statechart::statechart_update_context::StatechartUpdateContext;

#[derive(Default)]
struct A {}

impl CustomStateTrait for A {
    fn get_children(&self) -> Vec<State> {
        vec![
            State::new(Box::new(A_A::default())),
            State::new(Box::new(A_B::default())),
        ]
    }
}

#[derive(Default)]
struct A_A {}

impl CustomStateTrait for A_A {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        context.transition(Box::new(A_B::default()));
    }
}

#[derive(Default)]
struct A_B {}

impl CustomStateTrait for A_B {}

#[test]
fn test_basic_transition() {
    let mut statechart = Statechart::new(Box::new(A::default()));
    statechart.update();
    assert_eq!(statechart.get_recursive_active_type_ids(), vec![TypeId::of::<A_B>()]);
}
