use std::any::TypeId;
use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_elapsed::Timer_Elapsed;
use crate::timer_running::Timer_Running;

#[allow(non_camel_case_types)]
pub struct Timer_Root {}

impl CustomStateTrait for Timer_Root {
    fn new() -> Self {
        Timer_Root {}
    }

    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_children_and_type_ids() -> (Vec<State>, Vec<TypeId>) {
        let children: Vec<State> = vec![
            State::new::<Timer_Running>(),
            State::new::<Timer_Elapsed>(),
        ];

        let children_type_ids: Vec<TypeId> = vec![
            TypeId::of::<Timer_Running>(),
            TypeId::of::<Timer_Elapsed>(),
        ];

        (children, children_type_ids)
    }
}
