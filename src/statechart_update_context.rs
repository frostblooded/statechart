use std::any::TypeId;

pub struct StatechartUpdateContext {
    pub transitions: Vec<TypeId>
}

impl StatechartUpdateContext {
    pub fn new() -> Self {
        StatechartUpdateContext {
            transitions: vec![]
        }
    }

    pub fn transition<NewStateType: 'static>(&mut self) {
        let new_state_type_id = TypeId::of::<NewStateType>();
        self.transitions.push(new_state_type_id);
    }
}
