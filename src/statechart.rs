use std::any::TypeId;
use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;

pub struct Statechart {
    root: State,
}

impl Statechart {
    pub fn new<T: CustomStateTrait + 'static>(custom_state: Box<T>) -> Statechart {
        Self {
            root: State::new(custom_state),
        }
    }

    pub fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
        self.root.apply_transitions(context);
    }

    pub fn get_recursive_active_type_ids(&self) -> Vec<TypeId> {
        self.root.get_recursive_active_type_ids()
    }
}
