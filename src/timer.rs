use crate::state::State;
use crate::state_trait::StateTrait;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_root::Timer_Root;

pub struct Timer {
    pub root: State,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            root: State::new::<Timer_Root>(),
        }
    }

    pub fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
        self.root.handle_transitions(context);
    }
}
