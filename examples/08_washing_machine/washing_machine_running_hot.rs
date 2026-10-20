use crate::washing_machine_running::WashingMachine_Running;
use crate::washing_machine_running_heating::{HOT, WashingMachine_Running_Heating};
use statechart::prelude::*;

/// The heater is off: every update the water cools by 5 °C, and heating starts again once it
/// is more than 5 °C under `HOT`.
#[derive(Debug, Default)]
pub struct WashingMachine_Running_Hot;

impl State for WashingMachine_Running_Hot {
    type Parent = WashingMachine_Running;

    fn update(&mut self, ctx: &mut Context<Self>) {
        let running = ctx.parent_mut();
        running.temperature -= 5;
        if running.temperature < HOT - 5 {
            ctx.transition(WashingMachine_Running_Heating);
        }
    }
}
