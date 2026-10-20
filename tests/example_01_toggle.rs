// Runs the state code from the `01_toggle` example: a light with two states and one trigger.
#![allow(non_camel_case_types)]

#[path = "../examples/01_toggle/light.rs"]
mod light;
#[path = "../examples/01_toggle/light_off.rs"]
mod light_off;
#[path = "../examples/01_toggle/light_on.rs"]
mod light_on;
#[path = "../examples/01_toggle/triggers.rs"]
mod triggers;

use light::Light;
use light_off::Light_Off;
use light_on::Light_On;
use statechart::prelude::*;
use triggers::Toggle;

#[test]
fn test_light_starts_off() {
    let chart = Statechart::new(Light);
    assert_eq!(chart.get_debug_name(), "Light -> Light_Off");
    assert!(chart.is_active::<Light_Off>());
    assert!(!chart.is_active::<Light_On>());
}

#[test]
fn test_toggle_switches_between_off_and_on() {
    let mut chart = Statechart::new(Light);
    assert_eq!(chart.trigger(Toggle), Ok(()));
    assert_eq!(chart.get_debug_name(), "Light -> Light_On");
    assert_eq!(chart.trigger(Toggle), Ok(()));
    assert_eq!(chart.get_debug_name(), "Light -> Light_Off");
}

#[test]
fn test_update_does_nothing() {
    let mut chart = Statechart::new(Light);
    chart.update();
    assert_eq!(chart.get_debug_name(), "Light -> Light_Off");
}
