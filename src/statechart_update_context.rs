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

    pub fn transition<NewCustomStateType: 'static>(&mut self) {
        let new_custom_state_type_id = TypeId::of::<NewCustomStateType>();
        self.transitions.push(new_custom_state_type_id);
    }
}
