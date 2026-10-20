//! `cargo run --example 10_smart_home`: most of the library at once. Three parallel subsystems
//! at the root, one of which has two parallel regions of its own; triggers that bubble to a
//! shared ancestor; hooks that write to the root's log; `on_enter` hooks that request
//! transitions in other regions; a child built from the root's data (`region_with`) and one
//! entered with explicit data (`child_explicit`); `trigger_all`, introspection, and a restart
//! of the whole chart.
//!
//! ```text
//! SmartHome { comfort, indoor, log }
//! ├── region: SmartHome_Security (initial)
//! │   └── region: SmartHome_Security_Disarmed (initial), SmartHome_Security_Armed
//! │       SmartHome_Security_Armed
//! │       └── region: SmartHome_Security_Armed_Watching (initial),
//! │                   SmartHome_Security_Armed_Alarm { ticks }
//! ├── region: SmartHome_Climate { target } (initial, built from `comfort`)
//! │   ├── region: SmartHome_Climate_Idle (initial), SmartHome_Climate_Heating,
//! │   │           SmartHome_Climate_Cooling
//! │   └── region: SmartHome_Climate_FanAuto (initial), SmartHome_Climate_FanOn
//! └── region: SmartHome_LightsOff (initial), SmartHome_LightsOn { brightness } (explicit)
//! ```
#![allow(non_camel_case_types)]

mod smart_home;
mod smart_home_climate;
mod smart_home_climate_cooling;
mod smart_home_climate_fan_auto;
mod smart_home_climate_fan_on;
mod smart_home_climate_heating;
mod smart_home_climate_idle;
mod smart_home_lights_off;
mod smart_home_lights_on;
mod smart_home_security;
mod smart_home_security_armed;
mod smart_home_security_armed_alarm;
mod smart_home_security_armed_watching;
mod smart_home_security_disarmed;
mod triggers;

use smart_home::SmartHome;
use smart_home_climate::SmartHome_Climate;
use smart_home_security_armed_alarm::SmartHome_Security_Armed_Alarm;
use statechart::prelude::*;
use triggers::{Arrive, Describe, FactoryReset, Leave, Motion, SetTemperature, ToggleFan};

fn status(chart: &mut Statechart<SmartHome>) {
    let parts = chart
        .trigger_all(Describe)
        .expect("every subsystem answers");
    println!("  {}", parts.join(" | "));
}

fn main() {
    let mut chart = Statechart::new(SmartHome::new(21, 18));
    println!("{chart:?}");
    status(&mut chart);

    println!("-- a cold morning");
    for _ in 0..4 {
        chart.update();
        status(&mut chart);
    }
    println!("Motion -> {:?}", chart.trigger(Motion));
    println!("ToggleFan -> {:?}", chart.trigger(ToggleFan));
    println!(
        "SetTemperature(20) -> {:?}",
        chart.trigger(SetTemperature(20))
    );
    chart.update();
    chart.update();
    status(&mut chart);

    println!("-- everyone leaves");
    println!("Leave -> {:?}", chart.trigger(Leave));
    println!("{chart:?}");
    status(&mut chart);

    println!("-- a burglar");
    println!("Motion -> {:?}", chart.trigger_all(Motion));
    status(&mut chart);
    for _ in 0..3 {
        chart.update();
    }
    let alarm = chart
        .state::<SmartHome_Security_Armed_Alarm>()
        .expect("ringing");
    println!("  alarm ringing for {} updates", alarm.ticks);

    println!("-- back home");
    println!("Arrive -> {:?}", chart.trigger_all(Arrive));
    status(&mut chart);

    println!("log:");
    for line in &chart.root().log {
        println!("  {line}");
    }

    println!("-- factory reset");
    let target = chart
        .state::<SmartHome_Climate>()
        .expect("always active")
        .target;
    println!("  target before: {target}");
    println!("FactoryReset -> {:?}", chart.trigger(FactoryReset));
    println!("{chart:?}");
    status(&mut chart);
    println!("log: {:?}", chart.root().log);
}
