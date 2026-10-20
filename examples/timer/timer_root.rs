use crate::timer_elapsed::Timer_Elapsed;
use crate::timer_running::Timer_Running;
use statechart::custom_state_trait::CustomStateTrait;
use std::any::TypeId;
use std::time::Duration;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct Timer_Root {
    duration: Duration,
}

impl Timer_Root {
    pub fn new(duration: Duration) -> Self {
        Timer_Root { duration }
    }
}

impl CustomStateTrait for Timer_Root {
    fn get_possible_children_type_ids(&self) -> Vec<TypeId> {
        vec![TypeId::of::<Timer_Running>(), TypeId::of::<Timer_Elapsed>()]
    }

    fn get_initial_child(&self) -> Option<Box<dyn CustomStateTrait>> {
        Some(Box::new(Timer_Running::new(self.duration)))
    }
}
