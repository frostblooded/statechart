use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;

// TODO: Implement triggers
pub struct Statechart {
    root: State,
}

impl Statechart {
    pub fn new<T: CustomStateTrait>(custom_state: Box<T>) -> Statechart {
        let mut root: State = State::new(custom_state);
        root.on_enter();

        Self {
            root
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

    pub fn get_debug_name(&self) -> String {
        format!("{:?}", self.root)
    }
}

impl Debug for Statechart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
       write!(f, "{:?}", self.root)
    }
}
