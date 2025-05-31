use crate::custom_state_trait::CustomStateTrait;

#[derive(Default)]
pub struct StatechartUpdateContext {
    pub(crate) transitions: Vec<Box<dyn CustomStateTrait>>,
}

impl StatechartUpdateContext {
    pub fn new() -> Self {
        StatechartUpdateContext {
            ..Default::default()
        }
    }

    pub fn transition(&mut self, new_custom_state: Box<dyn CustomStateTrait>) {
        self.transitions.push(new_custom_state);
    }
}
