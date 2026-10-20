use crate::smart_home::SmartHome;
use crate::smart_home_lights_on::SmartHome_LightsOn;
use crate::smart_home_security_disarmed::SmartHome_Security_Disarmed;
use crate::triggers::{Arrive, Describe, Motion};
use statechart::prelude::*;

/// The lights are off.
#[derive(Debug, Default)]
pub struct SmartHome_LightsOff;

impl State for SmartHome_LightsOff {
    type Parent = SmartHome;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Arrive>()
            .trigger::<Motion>()
            .trigger::<Describe>();
    }
}

impl Trigger<Arrive> for SmartHome_LightsOff {
    // Runs alongside the security region's handler for the same trigger.
    fn handle(&mut self, _: &Arrive, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_LightsOn { brightness: 80 });
    }
}

impl Trigger<Motion> for SmartHome_LightsOff {
    // Someone home walks in: dim lights. While armed the alarm takes care of the lights.
    fn handle(&mut self, _: &Motion, ctx: &mut Context<Self>) {
        if ctx.is_active::<SmartHome_Security_Disarmed>() {
            ctx.transition(SmartHome_LightsOn { brightness: 50 });
        }
    }
}

impl Trigger<Describe> for SmartHome_LightsOff {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "lights off".to_string()
    }
}
