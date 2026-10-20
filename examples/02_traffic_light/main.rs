//! `cargo run --example 02_traffic_light`: states with their own data, changed by the `update`
//! hook. No triggers: each state counts its own ticks and moves on when its time is up.
//!
//! ```text
//! TrafficLight
//! └── region: TrafficLight_Red { ticks } (initial), TrafficLight_Green { ticks },
//!             TrafficLight_Yellow { ticks }
//! ```
#![allow(non_camel_case_types)]

mod traffic_light;
mod traffic_light_green;
mod traffic_light_red;
mod traffic_light_yellow;

use statechart::prelude::*;
use traffic_light::TrafficLight;

fn main() {
    let mut chart = Statechart::new(TrafficLight);
    println!("start:   {chart:?}");
    for tick in 1..=8 {
        // Runs `update` on every active state, then applies the requested transitions.
        chart.update();
        println!("tick {tick}:  {chart:?}");
    }
}
