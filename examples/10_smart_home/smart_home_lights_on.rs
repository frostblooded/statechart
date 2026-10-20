use crate::smart_home::SmartHome;
use crate::triggers::Describe;
use statechart::prelude::*;

/// The lights are on at `brightness` percent. Entered only with an explicit brightness.
#[derive(Debug)]
pub struct SmartHome_LightsOn {
    pub brightness: u32,
}

impl State for SmartHome_LightsOn {
    type Parent = SmartHome;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Describe>();
    }
}

impl Trigger<Describe> for SmartHome_LightsOn {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        format!("lights at {}%", self.brightness)
    }
}
