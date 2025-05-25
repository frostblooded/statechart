use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::timer_elapsed::Timer_Elapsed;
use crate::timer_running::Timer_Running;

#[allow(non_camel_case_types)]
pub struct Timer_Root {}

impl CustomStateTrait for Timer_Root {
    fn new() -> Self {
        Timer_Root {}
    }

    fn get_children() -> Vec<State> {
        vec![
            State::new::<Timer_Running>(),
            State::new::<Timer_Elapsed>(),
        ]
    }
}
