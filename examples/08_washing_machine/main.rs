//! `cargo run --example 08_washing_machine`: parallel regions. While running, the machine is in
//! one phase of the wash cycle *and* one thermostat state at the same time; each update runs
//! both, and a transition in one region leaves the other alone.
//!
//! ```text
//! WashingMachine
//! └── region: WashingMachine_Ready (initial), WashingMachine_Running { temperature },
//!             WashingMachine_Done
//!     WashingMachine_Running
//!     ├── region: WashingMachine_Running_Fill (initial), WashingMachine_Running_Wash,
//!     │           WashingMachine_Running_Rinse, WashingMachine_Running_Spin
//!     └── region: WashingMachine_Running_Heating (initial), WashingMachine_Running_Hot
//! ```
#![allow(non_camel_case_types)]

mod triggers;
mod washing_machine;
mod washing_machine_done;
mod washing_machine_ready;
mod washing_machine_running;
mod washing_machine_running_fill;
mod washing_machine_running_heating;
mod washing_machine_running_hot;
mod washing_machine_running_rinse;
mod washing_machine_running_spin;
mod washing_machine_running_wash;

use statechart::prelude::*;
use triggers::{Cancel, Start, Unload};
use washing_machine::WashingMachine;
use washing_machine_done::WashingMachine_Done;

fn main() {
    let mut chart = Statechart::new(WashingMachine);
    println!("{chart:?}");
    println!("Start -> {:?}", chart.trigger(Start));
    println!("  {chart:?}");
    while !chart.is_active::<WashingMachine_Done>() {
        chart.update();
        println!("  {chart:?}");
    }
    println!("Cancel -> {:?}", chart.trigger(Cancel));
    println!("Unload -> {:?}", chart.trigger(Unload));
    println!("  {chart:?}");

    // Cancelling exits both regions at once.
    println!("Start -> {:?}", chart.trigger(Start));
    chart.update();
    chart.update();
    chart.update();
    println!("  {chart:?}");
    println!("Cancel -> {:?}", chart.trigger(Cancel));
    println!("  {chart:?}");
}
