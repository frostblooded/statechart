use crate::state_behavior::StateBehavior;

#[allow(non_camel_case_types)]
pub struct Timer_Elapsed {}

impl Timer_Elapsed {
    pub fn new() -> Self {
        Timer_Elapsed {}
    }
}

impl StateBehavior for Timer_Elapsed {}
