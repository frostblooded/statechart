use crate::state_behavior::StateBehavior;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_meta_data::TimerMetaData;

pub struct Timer {
    pub meta_data: TimerMetaData,
}

impl Timer {
    pub fn new() -> Self {
        Timer { meta_data: TimerMetaData::new() }
    }
}

impl StateBehavior for Timer {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        for child_idx in &self.meta_data.active_children_idx {
            self.meta_data.children[*child_idx].update(context);
        }
    }
}
