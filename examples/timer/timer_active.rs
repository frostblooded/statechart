use crate::timer::Timer;
use crate::timer_active_paused::Timer_Active_Paused;
use crate::timer_active_running::Timer_Active_Running;
use crate::triggers::GetRemaining;
use statechart::prelude::*;
use std::fmt;
use std::time::Duration;

/// Counting down (running or paused). Holds the time left as of the last update.
pub struct Timer_Active {
    pub remaining: Duration,
}

// Rounds `remaining` to a tenth, so the active chain stays readable.
impl fmt::Debug for Timer_Active {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timer_Active {{ remaining: {:.1?} }}", self.remaining)
    }
}

impl State for Timer_Active {
    type Parent = Timer;

    fn define(def: &mut Definition<Self>) {
        def.region::<Timer_Active_Running>()
            .child::<Timer_Active_Paused>();
        def.trigger::<GetRemaining>();
    }
}

impl Trigger<GetRemaining> for Timer_Active {
    // The fallback for Timer_Active_Paused, which does not handle GetRemaining itself.
    fn handle(&mut self, _: &GetRemaining, _: &mut Context<Self>) -> Duration {
        self.remaining
    }
}
