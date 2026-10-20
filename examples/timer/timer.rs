use crate::timer_active::Timer_Active;
use crate::timer_elapsed::Timer_Elapsed;
use crate::triggers::Reset;
use statechart::prelude::*;
use std::time::Duration;

/// The root: a countdown of `duration`.
///
/// ```text
/// Timer { duration }
/// └── region: Timer_Active (initial, built from Timer), Timer_Elapsed
///     Timer_Active { remaining }
///     └── region: Timer_Active_Running (initial), Timer_Active_Paused
/// ```
#[derive(Debug)]
pub struct Timer {
    duration: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Timer { duration }
    }
}

impl State for Timer {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        // The initial child is built from the parent's data.
        def.region_with(|timer: &Timer| Timer_Active {
            remaining: timer.duration,
        })
        .child::<Timer_Elapsed>();
        def.trigger::<Reset>();
    }
}

impl Trigger<Reset> for Timer {
    fn handle(&mut self, _: &Reset, ctx: &mut Context<Self>) {
        // From Running or Paused this re-enters Timer_Active; from Elapsed it switches back.
        ctx.transition(Timer_Active {
            remaining: self.duration,
        });
    }
}
