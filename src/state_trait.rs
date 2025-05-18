use crate::custom_state_trait::CustomStateTrait;
use crate::statechart_update_context::StatechartUpdateContext;

pub trait StateTrait {
    fn new<T: CustomStateTrait + 'static>() -> Self where Self: Sized;
    fn update(&self, context: &mut StatechartUpdateContext) {}
}
