use crate::washing_machine_done::WashingMachine_Done;
use crate::washing_machine_running::WashingMachine_Running;
use statechart::prelude::*;

/// How many updates this phase lasts.
pub const DURATION: u32 = 2;

/// Spinning the water out. The last phase: it leaves Running, so the thermostat region stops too.
#[derive(Debug, Default)]
pub struct WashingMachine_Running_Spin {
    pub ticks: u32,
}

impl State for WashingMachine_Running_Spin {
    type Parent = WashingMachine_Running;

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == DURATION {
            ctx.transition(WashingMachine_Done);
        }
    }
}
