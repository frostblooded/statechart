use crate::washing_machine_running::WashingMachine_Running;
use crate::washing_machine_running_rinse::WashingMachine_Running_Rinse;
use statechart::prelude::*;

/// How many updates this phase lasts.
pub const DURATION: u32 = 3;

/// Tumbling the clothes in warm water.
#[derive(Debug, Default)]
pub struct WashingMachine_Running_Wash {
    pub ticks: u32,
}

impl State for WashingMachine_Running_Wash {
    type Parent = WashingMachine_Running;

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == DURATION {
            ctx.transition(WashingMachine_Running_Rinse::default());
        }
    }
}
