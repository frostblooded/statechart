use statechart::custom_state_trait::CustomStateTrait;

#[allow(non_camel_case_types)]
#[derive(Debug, Default)]
pub struct Timer_Elapsed {}

impl CustomStateTrait for Timer_Elapsed {}
