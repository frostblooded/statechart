use std::any::TypeId;

#[derive(Default)]
pub struct StatechartUpdateContext {
    pub(crate) transitions: Vec<TypeId>
}

impl StatechartUpdateContext {
    pub fn new() -> Self {
        StatechartUpdateContext {
            ..Default::default()
        }
    }

    pub fn transition<NewCustomStateType: 'static>(&mut self) {
        let new_custom_state_type_id = TypeId::of::<NewCustomStateType>();
        self.transitions.push(new_custom_state_type_id);
    }
}
