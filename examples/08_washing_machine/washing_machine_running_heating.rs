use crate::washing_machine_running::WashingMachine_Running;
use crate::washing_machine_running_hot::WashingMachine_Running_Hot;
use statechart::prelude::*;

/// The water is at least this hot (°C) when heating stops.
pub const HOT: u32 = 40;

/// The heater is on: every update warms the water by 10 °C, until it reaches `HOT`.
#[derive(Debug, Default)]
pub struct WashingMachine_Running_Heating;

impl State for WashingMachine_Running_Heating {
    type Parent = WashingMachine_Running;

    fn update(&mut self, ctx: &mut Context<Self>) {
        let running = ctx.parent_mut();
        running.temperature += 10;
        if running.temperature >= HOT {
            // Only this region changes; the cycle region keeps its phase.
            ctx.transition(WashingMachine_Running_Hot);
        }
    }
}
