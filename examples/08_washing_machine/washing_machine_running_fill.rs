use crate::washing_machine_running::WashingMachine_Running;
use crate::washing_machine_running_wash::WashingMachine_Running_Wash;
use statechart::prelude::*;

/// How many updates this phase lasts.
pub const DURATION: u32 = 2;

/// Letting water in.
#[derive(Debug, Default)]
pub struct WashingMachine_Running_Fill {
    pub ticks: u32,
}

impl State for WashingMachine_Running_Fill {
    type Parent = WashingMachine_Running;

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == DURATION {
            ctx.transition(WashingMachine_Running_Wash::default());
        }
    }
}
