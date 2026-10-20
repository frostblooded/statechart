//! `cargo run --example 04_door`: `on_enter` / `on_exit` hooks, and a state whose data is given
//! by the transition that enters it (`child_explicit`). The hooks write to the root's log
//! through `ctx.parent_mut()`.
//!
//! ```text
//! Door { log }
//! └── region: Door_Closed (initial), Door_Open, Door_Locked { code } (explicit)
//! ```
#![allow(non_camel_case_types)]

mod door;
mod door_closed;
mod door_locked;
mod door_open;
mod triggers;

use door::Door;
use statechart::prelude::*;
use triggers::{Close, Lock, Open, Unlock};

fn main() {
    let mut chart = Statechart::new(Door::default());
    println!("{chart:?}");

    println!("Open -> {:?}, {chart:?}", chart.trigger(Open));
    println!(
        "Lock -> {:?}, {chart:?}",
        chart.trigger(Lock { code: 1234 })
    );
    println!("Close -> {:?}, {chart:?}", chart.trigger(Close));
    println!(
        "Lock -> {:?}, {chart:?}",
        chart.trigger(Lock { code: 1234 })
    );
    println!("Open -> {:?}, {chart:?}", chart.trigger(Open));
    println!(
        "Unlock 1111 -> {:?}, {chart:?}",
        chart.trigger(Unlock { code: 1111 })
    );
    println!(
        "Unlock 1234 -> {:?}, {chart:?}",
        chart.trigger(Unlock { code: 1234 })
    );

    println!("log:");
    for line in &chart.root().log {
        println!("  {line}");
    }
}
