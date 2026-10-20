//! `cargo run --example 03_turnstile`: triggers that return a value, and `Err(NotHandled)`
//! when no active state handles a trigger.
//!
//! ```text
//! Turnstile
//! └── region: Turnstile_Locked (initial), Turnstile_Unlocked
//! ```
#![allow(non_camel_case_types)]

mod triggers;
mod turnstile;
mod turnstile_locked;
mod turnstile_unlocked;

use statechart::prelude::*;
use triggers::{Coin, Push};
use turnstile::Turnstile;

fn main() {
    let mut chart = Statechart::new(Turnstile);
    println!("{chart:?}");

    // A locked turnstile does not handle Push: the arm does not move.
    println!("Push -> {:?}, {chart:?}", chart.trigger(Push));
    println!("Coin -> {:?}, {chart:?}", chart.trigger(Coin));
    println!("Coin -> {:?}, {chart:?}", chart.trigger(Coin));
    println!("Push -> {:?}, {chart:?}", chart.trigger(Push));
}
