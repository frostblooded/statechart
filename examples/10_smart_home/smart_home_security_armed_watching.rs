use crate::smart_home_security_armed::SmartHome_Security_Armed;
use crate::smart_home_security_armed_alarm::SmartHome_Security_Armed_Alarm;
use crate::triggers::Motion;
use statechart::prelude::*;

/// Armed and quiet, watching the motion sensors.
#[derive(Debug, Default)]
pub struct SmartHome_Security_Armed_Watching;

impl State for SmartHome_Security_Armed_Watching {
    type Parent = SmartHome_Security_Armed;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Motion>();
    }
}

impl Trigger<Motion> for SmartHome_Security_Armed_Watching {
    fn handle(&mut self, _: &Motion, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_Security_Armed_Alarm::default());
    }
}
