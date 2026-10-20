use crate::smart_home::SmartHome;
use crate::smart_home_climate_cooling::SmartHome_Climate_Cooling;
use crate::smart_home_climate_fan_auto::SmartHome_Climate_FanAuto;
use crate::smart_home_climate_fan_on::SmartHome_Climate_FanOn;
use crate::smart_home_climate_heating::SmartHome_Climate_Heating;
use crate::smart_home_climate_idle::SmartHome_Climate_Idle;
use crate::triggers::{Describe, SetTemperature};
use statechart::prelude::*;

/// The heating and air conditioning subsystem, aiming for `target` °C. Two regions of its own
/// run in parallel: the mode (idle, heating, cooling) and the fan.
#[derive(Debug)]
pub struct SmartHome_Climate {
    pub target: i32,
}

impl State for SmartHome_Climate {
    type Parent = SmartHome;

    fn define(def: &mut Definition<Self>) {
        def.region::<SmartHome_Climate_Idle>()
            .child::<SmartHome_Climate_Heating>()
            .child::<SmartHome_Climate_Cooling>();
        def.region::<SmartHome_Climate_FanAuto>()
            .child::<SmartHome_Climate_FanOn>();
        def.trigger::<SetTemperature>().trigger::<Describe>();
    }
}

impl Trigger<SetTemperature> for SmartHome_Climate {
    // Only data changes; the mode region reacts on the next update.
    fn handle(&mut self, set: &SetTemperature, _: &mut Context<Self>) {
        self.target = set.0;
    }
}

impl Trigger<Describe> for SmartHome_Climate {
    // None of its children handle Describe, so both regions bubble up to here, and a shared
    // ancestor runs only once: one answer for the whole subsystem.
    fn handle(&mut self, _: &Describe, ctx: &mut Context<Self>) -> String {
        let heating = ctx.is_active::<SmartHome_Climate_Heating>();
        let cooling = ctx.is_active::<SmartHome_Climate_Cooling>();
        let mode = match (heating, cooling) {
            (true, _) => "heating",
            (_, true) => "cooling",
            _ => "idle",
        };
        let fan = if ctx.is_active::<SmartHome_Climate_FanOn>() {
            "fan on"
        } else if heating || cooling {
            "fan running"
        } else {
            "fan auto"
        };
        format!(
            "{}°C (target {}), {mode}, {fan}",
            ctx.parent().indoor,
            self.target
        )
    }
}
