use crate::custom_state_trait::CustomStateTrait;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;

pub struct Statechart {
    root: State,
}

impl Statechart {
    pub fn new<T: CustomStateTrait + 'static>() -> Statechart {
        Self {
            root: State::new::<T>(),
        }
    }

    pub fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
    }
}
