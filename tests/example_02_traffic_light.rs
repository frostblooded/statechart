// Runs the state code from the `02_traffic_light` example: `update` counts ticks in each
// state's own data and moves on once the state's duration is over.
#![allow(non_camel_case_types)]

#[path = "../examples/02_traffic_light/traffic_light.rs"]
mod traffic_light;
#[path = "../examples/02_traffic_light/traffic_light_green.rs"]
mod traffic_light_green;
#[path = "../examples/02_traffic_light/traffic_light_red.rs"]
mod traffic_light_red;
#[path = "../examples/02_traffic_light/traffic_light_yellow.rs"]
mod traffic_light_yellow;

use statechart::prelude::*;
use traffic_light::TrafficLight;
use traffic_light_green::TrafficLight_Green;
use traffic_light_red::TrafficLight_Red;
use traffic_light_yellow::TrafficLight_Yellow;

fn updates(chart: &mut Statechart<TrafficLight>, n: u32) {
    for _ in 0..n {
        chart.update();
    }
}

#[test]
fn test_starts_red_with_no_ticks() {
    let chart = Statechart::new(TrafficLight);
    assert_eq!(
        chart.get_debug_name(),
        "TrafficLight -> TrafficLight_Red { ticks: 0 }"
    );
}

#[test]
fn test_each_update_counts_a_tick_in_the_active_state() {
    let mut chart = Statechart::new(TrafficLight);
    updates(&mut chart, traffic_light_red::DURATION - 1);
    assert_eq!(
        chart.state::<TrafficLight_Red>().unwrap().ticks,
        traffic_light_red::DURATION - 1
    );
}

#[test]
fn test_cycles_red_green_yellow_red() {
    let mut chart = Statechart::new(TrafficLight);
    updates(&mut chart, traffic_light_red::DURATION);
    assert_eq!(
        chart.get_debug_name(),
        "TrafficLight -> TrafficLight_Green { ticks: 0 }"
    );
    assert!(chart.is_active::<TrafficLight_Green>());
    updates(&mut chart, traffic_light_green::DURATION);
    assert!(chart.is_active::<TrafficLight_Yellow>());
    updates(&mut chart, traffic_light_yellow::DURATION);
    // A fresh Red: its tick count starts over.
    assert_eq!(
        chart.get_debug_name(),
        "TrafficLight -> TrafficLight_Red { ticks: 0 }"
    );
}
