use std::any::TypeId;
use crate::state_behavior::StateBehavior;
use crate::timer_elapsed::Timer_Elapsed;
use crate::timer_running::Timer_Running;

pub struct TimerMetaData {
    pub children_type_ids: Vec<TypeId>,
    pub children: Vec<Box<dyn StateBehavior>>,
    pub active_children_idx: Vec<usize>
}

impl TimerMetaData {
    pub fn new() -> Self {
        let default_state: Box<Timer_Running> = Box::new(Timer_Running::new());

        let children: Vec<Box<dyn StateBehavior>> = vec![
            default_state,
            Box::new(Timer_Elapsed::new()),
        ];

        let children_type_ids: Vec<TypeId> = vec![
            TypeId::of::<Timer_Running>(),
            TypeId::of::<Timer_Elapsed>(),
        ];

        TimerMetaData { children, active_children_idx: vec![0], children_type_ids }
    }
}
