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
}
