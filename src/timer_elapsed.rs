use crate::custom_state_trait::CustomStateTrait;

#[allow(non_camel_case_types)]
pub struct Timer_Elapsed {}

impl CustomStateTrait for Timer_Elapsed {
    fn new() -> Self {
        Self {}
    }
}
