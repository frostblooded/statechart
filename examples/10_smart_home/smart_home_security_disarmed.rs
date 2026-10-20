use crate::smart_home_security::SmartHome_Security;
use crate::smart_home_security_armed::SmartHome_Security_Armed;
use crate::triggers::{Describe, Leave};
use statechart::prelude::*;

/// Someone is home.
#[derive(Debug, Default)]
pub struct SmartHome_Security_Disarmed;

impl State for SmartHome_Security_Disarmed {
    type Parent = SmartHome_Security;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Leave>().trigger::<Describe>();
    }
}

impl Trigger<Leave> for SmartHome_Security_Disarmed {
    fn handle(&mut self, _: &Leave, ctx: &mut Context<Self>) {
        ctx.transition(SmartHome_Security_Armed);
    }
}

impl Trigger<Describe> for SmartHome_Security_Disarmed {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "disarmed".to_string()
    }
}
