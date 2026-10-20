use crate::timer_active::Timer_Active;
use crate::timer_active_paused::Timer_Active_Paused;
use crate::timer_elapsed::Timer_Elapsed;
use crate::triggers::{GetRemaining, Pause};
use statechart::prelude::*;
use std::fmt;
use std::time::{Duration, Instant};

/// The clock is running: every update takes the time since the last one off the parent's
/// `remaining`.
pub struct Timer_Active_Running {
    last_tick: Instant,
}

impl Default for Timer_Active_Running {
    fn default() -> Self {
        Timer_Active_Running {
            last_tick: Instant::now(),
        }
    }
}

impl Timer_Active_Running {
    /// Moves the time since the last tick from the parent's `remaining`.
    fn tick(&mut self, active: &mut Timer_Active) {
        let now = Instant::now();
        active.remaining = active
            .remaining
            .saturating_sub(now.duration_since(self.last_tick));
        self.last_tick = now;
    }
}

// Only the name: an `Instant` is noise in the active chain.
impl fmt::Debug for Timer_Active_Running {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Timer_Active_Running")
    }
}

impl State for Timer_Active_Running {
    type Parent = Timer_Active;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Pause>().trigger::<GetRemaining>();
    }

    fn on_enter(&mut self, _: &mut Context<Self>) {
        self.last_tick = Instant::now();
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.tick(ctx.parent_mut());
        if ctx.parent().remaining.is_zero() {
            // Cross-level: Timer_Elapsed is a sibling of the parent.
            ctx.transition(Timer_Elapsed);
        }
    }
}

impl Trigger<Pause> for Timer_Active_Running {
    fn handle(&mut self, _: &Pause, ctx: &mut Context<Self>) {
        self.tick(ctx.parent_mut());
        ctx.transition(Timer_Active_Paused);
    }
}

impl Trigger<GetRemaining> for Timer_Active_Running {
    // The live value; the parent's handler is not asked.
    fn handle(&mut self, _: &GetRemaining, ctx: &mut Context<Self>) -> Duration {
        ctx.parent()
            .remaining
            .saturating_sub(self.last_tick.elapsed())
    }
}
