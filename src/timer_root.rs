use std::any::TypeId;
use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_elapsed::Timer_Elapsed;
use crate::timer_running::Timer_Running;

macro_rules! state_children {
    ($($a:ty),*) => {
        (
            vec![
                $(
                    State::new::<$a>(),
                )*
            ],
            vec![
                $(
                    TypeId::of::<$a>(),
                )*
            ]
        )
    };
}

#[allow(non_camel_case_types)]
pub struct Timer_Root {}

impl CustomStateTrait for Timer_Root {
    fn new() -> Self {
        Timer_Root {}
    }

    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_children_and_type_ids() -> (Vec<State>, Vec<TypeId>) {
        state_children!(Timer_Running, Timer_Elapsed)
    }
}
