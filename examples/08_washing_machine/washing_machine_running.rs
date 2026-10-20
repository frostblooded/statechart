use crate::triggers::Cancel;
use crate::washing_machine::WashingMachine;
use crate::washing_machine_ready::WashingMachine_Ready;
use crate::washing_machine_running_fill::WashingMachine_Running_Fill;
use crate::washing_machine_running_heating::WashingMachine_Running_Heating;
use crate::washing_machine_running_hot::WashingMachine_Running_Hot;
use crate::washing_machine_running_rinse::WashingMachine_Running_Rinse;
use crate::washing_machine_running_spin::WashingMachine_Running_Spin;
use crate::washing_machine_running_wash::WashingMachine_Running_Wash;
use statechart::prelude::*;

/// Washing. Two regions run in parallel: the cycle and the thermostat, which keeps the
/// water's `temperature` (in °C) near 40.
#[derive(Debug)]
pub struct WashingMachine_Running {
    pub temperature: u32,
}

impl Default for WashingMachine_Running {
    fn default() -> Self {
        // Cold tap water.
        WashingMachine_Running { temperature: 20 }
    }
}

impl State for WashingMachine_Running {
    type Parent = WashingMachine;

    fn define(def: &mut Definition<Self>) {
        // Two `region` calls: two regions, each with its own active child.
        def.region::<WashingMachine_Running_Fill>()
            .child::<WashingMachine_Running_Wash>()
            .child::<WashingMachine_Running_Rinse>()
            .child::<WashingMachine_Running_Spin>();
        def.region::<WashingMachine_Running_Heating>()
            .child::<WashingMachine_Running_Hot>();
        def.trigger::<Cancel>();
    }
}

impl Trigger<Cancel> for WashingMachine_Running {
    // Leaving Running exits the active state of both regions.
    fn handle(&mut self, _: &Cancel, ctx: &mut Context<Self>) {
        ctx.transition(WashingMachine_Ready);
    }
}
