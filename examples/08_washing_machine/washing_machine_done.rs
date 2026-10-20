use crate::triggers::Unload;
use crate::washing_machine::WashingMachine;
use crate::washing_machine_ready::WashingMachine_Ready;
use statechart::prelude::*;

/// The cycle is over; `Unload` makes the machine ready again.
#[derive(Debug, Default)]
pub struct WashingMachine_Done;

impl State for WashingMachine_Done {
    type Parent = WashingMachine;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Unload>();
    }
}

impl Trigger<Unload> for WashingMachine_Done {
    fn handle(&mut self, _: &Unload, ctx: &mut Context<Self>) {
        ctx.transition(WashingMachine_Ready);
    }
}
