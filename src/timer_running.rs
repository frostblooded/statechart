use std::time::{Duration, Instant};
use crate::state_behavior::StateBehavior;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_elapsed::Timer_Elapsed;

#[allow(non_camel_case_types)]
pub struct Timer_Running {
    duration: Duration,
    start_time: Instant,
}

impl Timer_Running {
    pub fn new() -> Self {
        Timer_Running { duration: Duration::from_secs(3), start_time: Instant::now() }
    }
}

impl StateBehavior for Timer_Running {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let now: Instant = Instant::now();
        let elapsed_time: Duration = now.duration_since(self.start_time);

        if elapsed_time >= self.duration {
            self.transition::<Timer_Elapsed>(context);
        }
    }
}
