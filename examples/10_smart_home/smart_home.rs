use crate::smart_home_climate::SmartHome_Climate;
use crate::smart_home_lights_off::SmartHome_LightsOff;
use crate::smart_home_lights_on::SmartHome_LightsOn;
use crate::smart_home_security::SmartHome_Security;
use crate::triggers::FactoryReset;
use statechart::prelude::*;
use std::fmt;

/// The root: the preferred temperature (`comfort`), the temperature in the house (`indoor`,
/// both in °C) and a `log` that the hooks of every subsystem write to.
pub struct SmartHome {
    pub comfort: i32,
    pub indoor: i32,
    pub log: Vec<String>,
}

impl SmartHome {
    pub fn new(comfort: i32, indoor: i32) -> Self {
        SmartHome {
            comfort,
            indoor,
            log: Vec::new(),
        }
    }

    /// Adds a line to the log.
    pub fn note(&mut self, line: &str) {
        self.log.push(line.to_string());
    }
}

// Leaves the log out of the active configuration.
impl fmt::Debug for SmartHome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SmartHome {{ comfort: {}, indoor: {} }}",
            self.comfort, self.indoor
        )
    }
}

impl State for SmartHome {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        // Three regions: the subsystems run side by side.
        def.region::<SmartHome_Security>();
        def.region_with(|home: &SmartHome| SmartHome_Climate {
            target: home.comfort,
        });
        def.region::<SmartHome_LightsOff>()
            .child_explicit::<SmartHome_LightsOn>();
        def.trigger::<FactoryReset>();
    }

    fn on_enter(&mut self, _: &mut Context<Self>) {
        self.note("system started");
    }
}

impl Trigger<FactoryReset> for SmartHome {
    fn handle(&mut self, _: &FactoryReset, ctx: &mut Context<Self>) {
        // Targeting the root restarts the chart: everything exits and this new root is
        // entered with its initial children. Only `comfort` and `indoor` carry over.
        ctx.transition(SmartHome::new(self.comfort, self.indoor));
    }
}
