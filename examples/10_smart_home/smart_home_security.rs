use crate::smart_home::SmartHome;
use crate::smart_home_security_armed::SmartHome_Security_Armed;
use crate::smart_home_security_disarmed::SmartHome_Security_Disarmed;
use statechart::prelude::*;

/// The burglar alarm subsystem.
#[derive(Debug, Default)]
pub struct SmartHome_Security;

impl State for SmartHome_Security {
    type Parent = SmartHome;

    fn define(def: &mut Definition<Self>) {
        def.region::<SmartHome_Security_Disarmed>()
            .child::<SmartHome_Security_Armed>();
    }
}
