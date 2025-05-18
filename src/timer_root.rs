use crate::custom_state_trait::CustomStateTrait;
use crate::statechart_update_context::StatechartUpdateContext;

#[allow(non_camel_case_types)]
pub struct Timer_Root {}

impl CustomStateTrait for Timer_Root {
    fn new() -> Self {
        Timer_Root {}
    }

    fn update(&mut self, context: &mut StatechartUpdateContext) {}
}
