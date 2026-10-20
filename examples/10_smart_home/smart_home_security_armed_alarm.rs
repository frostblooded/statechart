use crate::smart_home::SmartHome;
use crate::smart_home_lights_on::SmartHome_LightsOn;
use crate::smart_home_security_armed::SmartHome_Security_Armed;
use crate::triggers::Describe;
use statechart::prelude::*;

/// After how many updates of ringing the police is called.
pub const POLICE_AFTER: u32 = 3;

/// The siren is on; it has been ringing for `ticks` updates.
#[derive(Debug, Default)]
pub struct SmartHome_Security_Armed_Alarm {
    pub ticks: u32,
}

impl State for SmartHome_Security_Armed_Alarm {
    type Parent = SmartHome_Security_Armed;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Describe>();
    }

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        ctx.ancestor_mut::<SmartHome>()
            .expect("the root")
            .note("intruder detected");
        // Every light at full brightness, in the lights region.
        ctx.transition(SmartHome_LightsOn { brightness: 100 });
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == POLICE_AFTER {
            ctx.ancestor_mut::<SmartHome>()
                .expect("the root")
                .note("calling the police");
        }
    }
}

impl Trigger<Describe> for SmartHome_Security_Armed_Alarm {
    // Deeper than Armed, so this answer wins for the security region.
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "ALARM".to_string()
    }
}
