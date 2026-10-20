//! `cargo run --example 07_elevator`: four levels of nesting. A deep state changes the root's
//! data with `ctx.ancestor_mut`, children are built from their parent's data (`child_with`),
//! and transitions jump straight into states whose ancestors are not active yet: the library
//! builds those ancestors on the way down.
//!
//! ```text
//! Elevator { floor, top_floor }
//! └── region: Elevator_Service { queue } (initial), Elevator_Maintenance
//!     Elevator_Service
//!     └── region: Elevator_Service_Idle (initial), Elevator_Service_Moving { target }
//!                 (built from the queue), Elevator_Service_DoorsOpen { ticks }
//!         Elevator_Service_Moving
//!         └── region: Elevator_Service_Moving_Up (initial), Elevator_Service_Moving_Down
//! ```
#![allow(non_camel_case_types)]

mod elevator;
mod elevator_maintenance;
mod elevator_service;
mod elevator_service_doors_open;
mod elevator_service_idle;
mod elevator_service_moving;
mod elevator_service_moving_down;
mod elevator_service_moving_up;
mod triggers;

use elevator::Elevator;
use statechart::prelude::*;
use triggers::{Call, Resume, StartMaintenance};

fn run(chart: &mut Statechart<Elevator>, updates: u32) {
    for _ in 0..updates {
        chart.update();
        println!("  {chart:?}");
    }
}

fn main() {
    let mut chart = Statechart::new(Elevator::new(5));
    println!("{chart:?}");

    println!("Call(9) -> {:?}", chart.trigger(Call(9)));
    println!("Call(3) -> {:?}", chart.trigger(Call(3)));
    println!("Call(1) -> {:?}", chart.trigger(Call(1)));
    run(&mut chart, 10);

    println!("Call(4) -> {:?}", chart.trigger(Call(4)));
    run(&mut chart, 3);
    println!("StartMaintenance -> {:?}", chart.trigger(StartMaintenance));
    println!("  {chart:?}");
    println!("Call(2) -> {:?}", chart.trigger(Call(2)));
    // Straight into Moving_Down: Service and Moving are built on the way.
    println!("Resume -> {:?}", chart.trigger(Resume));
    println!("  {chart:?}");
    run(&mut chart, 4);
}
