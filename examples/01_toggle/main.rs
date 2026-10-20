//! `cargo run --example 01_toggle`: the smallest statechart, a light with two states and one
//! trigger. Start here.
//!
//! ```text
//! Light
//! └── region: Light_Off (initial), Light_On
//! ```
#![allow(non_camel_case_types)]

mod light;
mod light_off;
mod light_on;
mod triggers;

use light::Light;
use statechart::prelude::*;
use triggers::Toggle;

fn main() {
    let mut chart = Statechart::new(Light);
    println!("{chart:?}");
    for _ in 0..3 {
        chart.trigger(Toggle).expect("both states handle Toggle");
        println!("Toggle -> {chart:?}");
    }
}
