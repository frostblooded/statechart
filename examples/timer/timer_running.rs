use statechart::custom_state_trait::CustomStateTrait;
use statechart::statechart_update_context::StatechartUpdateContext;
use crate::timer_elapsed::Timer_Elapsed;
use std::time::{Duration, Instant};

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct Timer_Running {
    duration: Duration,
    start_time: Instant,
}

impl Timer_Running {
    pub fn new(duration: Duration) -> Self {
        Timer_Running { duration, start_time: Instant::now() }
    }
}

impl CustomStateTrait for Timer_Running {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let now: Instant = Instant::now();
        let elapsed_time: Duration = now.duration_since(self.start_time);

        if elapsed_time >= self.duration {
            context.transition(Box::new(Timer_Elapsed::default()));
        }
    }
}
