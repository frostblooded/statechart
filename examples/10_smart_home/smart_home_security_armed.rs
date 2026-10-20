use crate::smart_home::SmartHome;
use crate::smart_home_lights_off::SmartHome_LightsOff;
use crate::smart_home_lights_on::SmartHome_LightsOn;
use crate::smart_home_security::SmartHome_Security;
use crate::smart_home_security_armed_alarm::SmartHome_Security_Armed_Alarm;
use crate::smart_home_security_armed_watching::SmartHome_Security_Armed_Watching;
use crate::smart_home_security_disarmed::SmartHome_Security_Disarmed;
use crate::triggers::{Arrive, Describe};
use statechart::prelude::*;

/// Nobody is home. The fallback for both children: `Arrive` disarms whether the alarm is
/// ringing or not, and `Describe` answers "armed" unless the alarm answers first.
#[derive(Debug, Default)]
pub struct SmartHome_Security_Armed;

impl State for SmartHome_Security_Armed {
    type Parent = SmartHome_Security;

    fn define(def: &mut Definition<Self>) {
        def.region::<SmartHome_Security_Armed_Watching>()
            .child::<SmartHome_Security_Armed_Alarm>();
        def.trigger::<Arrive>().trigger::<Describe>();
    }

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        ctx.ancestor_mut::<SmartHome>()
            .expect("the root")
            .note("alarm armed");
        // Requested from on_enter, for the lights region: applied later in the same batch.
        if ctx.is_active::<SmartHome_LightsOn>() {
            ctx.transition(SmartHome_LightsOff);
        }
    }

    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        ctx.ancestor_mut::<SmartHome>()
            .expect("the root")
            .note("alarm disarmed");
    }
}

impl Trigger<Arrive> for SmartHome_Security_Armed {
    fn handle(&mut self, _: &Arrive, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_Security_Disarmed);
    }
}

impl Trigger<Describe> for SmartHome_Security_Armed {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "armed".to_string()
    }
}
