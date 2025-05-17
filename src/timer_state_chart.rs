use std::any::TypeId;
use crate::state_behavior::StateBehavior;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::Timer;

pub struct TimerStateChart {
    pub root: Timer,
}

impl TimerStateChart {
    pub fn new() -> Self {
        TimerStateChart {
            root: Timer::new()
        }
    }

    pub fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
        self.handle_transitions(context);
    }

    fn handle_transitions(&mut self, context: StatechartUpdateContext) {
        for transition_id in context.transitions {
            for idx in 0..self.root.meta_data.children.len() {
                let type_id: TypeId = self.root.meta_data.children_type_ids[idx];

                if type_id == transition_id {
                    self.root.meta_data.active_children_idx = vec![idx];
                }
            }
        }
    }
}
