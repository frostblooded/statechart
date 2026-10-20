use crate::triggers::Start;
use crate::washing_machine::WashingMachine;
use crate::washing_machine_running::WashingMachine_Running;
use statechart::prelude::*;

/// Loaded and waiting for `Start`.
#[derive(Debug, Default)]
pub struct WashingMachine_Ready;

impl State for WashingMachine_Ready {
    type Parent = WashingMachine;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Start>();
    }
}

impl Trigger<Start> for WashingMachine_Ready {
    fn handle(&mut self, _: &Start, ctx: &mut Context<Self>) {
        // Enters Running and the initial child of each of its two regions.
        ctx.transition(WashingMachine_Running::default());
    }
}
