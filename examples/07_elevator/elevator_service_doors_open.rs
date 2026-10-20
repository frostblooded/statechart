use crate::elevator_service::Elevator_Service;
use crate::elevator_service_idle::Elevator_Service_Idle;
use statechart::prelude::*;

/// How many updates the doors stay open.
pub const OPEN_TICKS: u32 = 2;

/// Arrived: the doors are open for `OPEN_TICKS` updates.
#[derive(Debug, Default)]
pub struct Elevator_Service_DoorsOpen {
    pub ticks: u32,
}

impl State for Elevator_Service_DoorsOpen {
    type Parent = Elevator_Service;

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        // This floor's call is served.
        ctx.parent_mut().queue.pop_front();
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == OPEN_TICKS {
            ctx.transition(Elevator_Service_Idle);
        }
    }
}
