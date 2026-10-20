// Runs the state code from the `10_smart_home` example, which uses most of the library at once:
// parallel regions at the root and nested inside a region, trigger fallback, hooks writing to
// the root's log, `on_enter` requests for other regions, data built from the parent, `trigger_all`,
// introspection and a restart of the whole chart.
#![allow(non_camel_case_types)]

#[path = "../examples/10_smart_home/smart_home.rs"]
mod smart_home;
#[path = "../examples/10_smart_home/smart_home_climate.rs"]
mod smart_home_climate;
#[path = "../examples/10_smart_home/smart_home_climate_cooling.rs"]
mod smart_home_climate_cooling;
#[path = "../examples/10_smart_home/smart_home_climate_fan_auto.rs"]
mod smart_home_climate_fan_auto;
#[path = "../examples/10_smart_home/smart_home_climate_fan_on.rs"]
mod smart_home_climate_fan_on;
#[path = "../examples/10_smart_home/smart_home_climate_heating.rs"]
mod smart_home_climate_heating;
#[path = "../examples/10_smart_home/smart_home_climate_idle.rs"]
mod smart_home_climate_idle;
#[path = "../examples/10_smart_home/smart_home_lights_off.rs"]
mod smart_home_lights_off;
#[path = "../examples/10_smart_home/smart_home_lights_on.rs"]
mod smart_home_lights_on;
#[path = "../examples/10_smart_home/smart_home_security.rs"]
mod smart_home_security;
#[path = "../examples/10_smart_home/smart_home_security_armed.rs"]
mod smart_home_security_armed;
#[path = "../examples/10_smart_home/smart_home_security_armed_alarm.rs"]
mod smart_home_security_armed_alarm;
#[path = "../examples/10_smart_home/smart_home_security_armed_watching.rs"]
mod smart_home_security_armed_watching;
#[path = "../examples/10_smart_home/smart_home_security_disarmed.rs"]
mod smart_home_security_disarmed;
#[path = "../examples/10_smart_home/triggers.rs"]
mod triggers;

use smart_home::SmartHome;
use smart_home_climate::SmartHome_Climate;
use smart_home_climate_cooling::SmartHome_Climate_Cooling;
use smart_home_climate_fan_auto::SmartHome_Climate_FanAuto;
use smart_home_climate_fan_on::SmartHome_Climate_FanOn;
use smart_home_climate_heating::SmartHome_Climate_Heating;
use smart_home_climate_idle::SmartHome_Climate_Idle;
use smart_home_lights_off::SmartHome_LightsOff;
use smart_home_lights_on::SmartHome_LightsOn;
use smart_home_security_armed::SmartHome_Security_Armed;
use smart_home_security_armed_alarm::SmartHome_Security_Armed_Alarm;
use smart_home_security_armed_watching::SmartHome_Security_Armed_Watching;
use smart_home_security_disarmed::SmartHome_Security_Disarmed;
use statechart::prelude::*;
use std::any::TypeId;
use triggers::{Arrive, Describe, FactoryReset, Leave, Motion, SetTemperature, ToggleFan};

fn home() -> Statechart<SmartHome> {
    Statechart::new(SmartHome::new(21, 21))
}

fn describe_all(chart: &mut Statechart<SmartHome>) -> Vec<String> {
    chart.trigger_all(Describe).unwrap()
}

#[test]
fn test_starts_with_three_parallel_subsystems() {
    let mut chart = home();
    assert_eq!(
        chart.get_debug_name(),
        "SmartHome { comfort: 21, indoor: 21 } -> [SmartHome_Security -> \
         SmartHome_Security_Disarmed, SmartHome_Climate { target: 21 } -> \
         [SmartHome_Climate_Idle, SmartHome_Climate_FanAuto], SmartHome_LightsOff]"
    );
    assert_eq!(chart.root().log, ["system started"]);
    // Climate answers once for both of its regions.
    assert_eq!(
        describe_all(&mut chart),
        ["disarmed", "21°C (target 21), idle, fan auto", "lights off"]
    );
}

#[test]
fn test_active_type_ids_are_post_order() {
    let chart = home();
    assert_eq!(
        chart.get_recursive_active_type_ids(),
        [
            TypeId::of::<SmartHome_Security_Disarmed>(),
            TypeId::of::<smart_home_security::SmartHome_Security>(),
            TypeId::of::<SmartHome_Climate_Idle>(),
            TypeId::of::<SmartHome_Climate_FanAuto>(),
            TypeId::of::<SmartHome_Climate>(),
            TypeId::of::<SmartHome_LightsOff>(),
        ]
    );
}

#[test]
fn test_climate_target_is_built_from_the_root_comfort() {
    let chart = Statechart::new(SmartHome::new(19, 25));
    assert_eq!(chart.state::<SmartHome_Climate>().unwrap().target, 19);
}

#[test]
fn test_heating_warms_the_house_to_the_target() {
    let mut chart = Statechart::new(SmartHome::new(21, 19));
    chart.update();
    assert!(chart.is_active::<SmartHome_Climate_Heating>());
    assert_eq!(chart.root().log, ["system started", "heating on"]);
    chart.update();
    assert_eq!(chart.root().indoor, 20);
    assert_eq!(
        describe_all(&mut chart)[1],
        "20°C (target 21), heating, fan running"
    );
    chart.update();
    assert_eq!(chart.root().indoor, 21);
    assert!(chart.is_active::<SmartHome_Climate_Idle>());
    assert_eq!(
        chart.root().log,
        ["system started", "heating on", "heating off"]
    );
}

#[test]
fn test_set_temperature_makes_it_cool() {
    let mut chart = home();
    chart.trigger(SetTemperature(20)).unwrap();
    assert_eq!(chart.state::<SmartHome_Climate>().unwrap().target, 20);
    chart.update();
    assert!(chart.is_active::<SmartHome_Climate_Cooling>());
    chart.update();
    assert_eq!(chart.root().indoor, 20);
    assert!(chart.is_active::<SmartHome_Climate_Idle>());
}

#[test]
fn test_the_fan_region_toggles_independently() {
    let mut chart = home();
    chart.trigger(ToggleFan).unwrap();
    assert!(chart.is_active::<SmartHome_Climate_FanOn>());
    assert!(chart.is_active::<SmartHome_Climate_Idle>());
    assert_eq!(
        describe_all(&mut chart)[1],
        "21°C (target 21), idle, fan on"
    );
    chart.trigger(ToggleFan).unwrap();
    assert!(chart.is_active::<SmartHome_Climate_FanAuto>());
}

#[test]
fn test_motion_while_home_turns_the_lights_on() {
    let mut chart = home();
    chart.trigger(Motion).unwrap();
    assert_eq!(chart.state::<SmartHome_LightsOn>().unwrap().brightness, 50);
    // Already on: nobody handles it.
    assert_eq!(chart.trigger(Motion), Err(NotHandled));
}

#[test]
fn test_leaving_arms_the_alarm_and_turns_the_lights_off() {
    let mut chart = home();
    chart.trigger(Motion).unwrap();
    chart.trigger(Leave).unwrap();
    assert!(chart.is_active::<SmartHome_Security_Armed_Watching>());
    // Armed's on_enter asked for the lights region to change.
    assert!(chart.is_active::<SmartHome_LightsOff>());
    assert_eq!(chart.root().log, ["system started", "alarm armed"]);
    assert_eq!(
        describe_all(&mut chart),
        ["armed", "21°C (target 21), idle, fan auto", "lights off"]
    );
    assert_eq!(chart.trigger(Leave), Err(NotHandled));
}

#[test]
fn test_motion_while_armed_sounds_the_alarm_with_full_lights() {
    let mut chart = home();
    chart.trigger(Leave).unwrap();
    chart.trigger(Motion).unwrap();
    assert!(chart.is_active::<SmartHome_Security_Armed_Alarm>());
    assert_eq!(chart.state::<SmartHome_LightsOn>().unwrap().brightness, 100);
    assert_eq!(describe_all(&mut chart)[0], "ALARM");
    for _ in 0..smart_home_security_armed_alarm::POLICE_AFTER {
        chart.update();
    }
    assert_eq!(
        chart.root().log,
        [
            "system started",
            "alarm armed",
            "intruder detected",
            "calling the police"
        ]
    );
    // Only once.
    chart.update();
    assert_eq!(chart.root().log.len(), 4);
}

#[test]
fn test_arriving_disarms_from_watching_or_alarm_and_lights_up() {
    let mut chart = home();
    chart.trigger(Leave).unwrap();
    // Two regions handle Arrive: Armed (for Watching) and LightsOff.
    assert_eq!(chart.trigger_all(Arrive), Ok(vec![(), ()]));
    assert!(chart.is_active::<SmartHome_Security_Disarmed>());
    assert_eq!(chart.state::<SmartHome_LightsOn>().unwrap().brightness, 80);
    assert_eq!(
        chart.root().log,
        ["system started", "alarm armed", "alarm disarmed"]
    );

    chart.trigger(Leave).unwrap();
    chart.trigger(Motion).unwrap();
    // Armed is also the fallback for Alarm; lights are already on, so only one handler runs.
    assert_eq!(chart.trigger_all(Arrive), Ok(vec![()]));
    assert!(chart.is_active::<SmartHome_Security_Disarmed>());
    assert_eq!(chart.state::<SmartHome_LightsOn>().unwrap().brightness, 100);
}

#[test]
fn test_factory_reset_restarts_the_whole_home() {
    let mut chart = Statechart::new(SmartHome::new(21, 25));
    chart.trigger(SetTemperature(18)).unwrap();
    chart.trigger(ToggleFan).unwrap();
    chart.trigger(Leave).unwrap();
    chart.update();
    chart.trigger(FactoryReset).unwrap();
    // A fresh root built from the old one's comfort and indoor temperature: the target set
    // with SetTemperature, the fan, the alarm and the log all start over.
    assert_eq!(
        chart.get_debug_name(),
        "SmartHome { comfort: 21, indoor: 25 } -> [SmartHome_Security -> \
         SmartHome_Security_Disarmed, SmartHome_Climate { target: 21 } -> \
         [SmartHome_Climate_Idle, SmartHome_Climate_FanAuto], SmartHome_LightsOff]"
    );
    assert_eq!(chart.root().log, ["system started"]);
    assert!(!chart.is_active::<SmartHome_Security_Armed>());
}
