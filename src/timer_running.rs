use crate::custom_state_trait::CustomStateTrait;
use crate::statechart_update_context::StatechartUpdateContext;
use crate::timer_elapsed::Timer_Elapsed;
use std::time::{Duration, Instant};

#[allow(non_camel_case_types)]
pub struct Timer_Running {
    duration: Duration,
    start_time: Instant,
}

impl CustomStateTrait for Timer_Running {
    fn new() -> Self {
        Timer_Running { duration: Duration::from_secs(3), start_time: Instant::now() }
    }

    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let now: Instant = Instant::now();
        let elapsed_time: Duration = now.duration_since(self.start_time);

        if elapsed_time >= self.duration {
            context.transition::<Timer_Elapsed>();
        }
    }
}
