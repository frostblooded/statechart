use crate::smart_home_climate::SmartHome_Climate;
use crate::smart_home_climate_fan_on::SmartHome_Climate_FanOn;
use crate::triggers::ToggleFan;
use statechart::prelude::*;

/// The fan runs only while heating or cooling.
#[derive(Debug, Default)]
pub struct SmartHome_Climate_FanAuto;

impl State for SmartHome_Climate_FanAuto {
    type Parent = SmartHome_Climate;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<ToggleFan>();
    }
}

impl Trigger<ToggleFan> for SmartHome_Climate_FanAuto {
    // Only the fan region changes; the mode region keeps heating, cooling or idling.
    fn handle(&mut self, _: &ToggleFan, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_Climate_FanOn);
    }
}
