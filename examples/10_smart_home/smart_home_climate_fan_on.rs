use crate::smart_home_climate::SmartHome_Climate;
use crate::smart_home_climate_fan_auto::SmartHome_Climate_FanAuto;
use crate::triggers::ToggleFan;
use statechart::prelude::*;

/// The fan runs all the time.
#[derive(Debug, Default)]
pub struct SmartHome_Climate_FanOn;

impl State for SmartHome_Climate_FanOn {
    type Parent = SmartHome_Climate;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<ToggleFan>();
    }
}

impl Trigger<ToggleFan> for SmartHome_Climate_FanOn {
    // Only the fan region changes; the mode region keeps heating, cooling or idling.
    fn handle(&mut self, _: &ToggleFan, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_Climate_FanAuto);
    }
}
