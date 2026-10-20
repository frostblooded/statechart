//! `cargo run --example 05_vending_machine`: nested states that read and write their
//! ancestors' data. The coins go into the parent's `credit` (`ctx.parent_mut()`); a sale takes
//! the price from the root and one item off its stock (`ctx.ancestor` / `ctx.ancestor_mut`).
//!
//! ```text
//! VendingMachine { price, stock }
//! └── region: VendingMachine_InService { credit } (initial), VendingMachine_SoldOut
//!     VendingMachine_InService
//!     └── region: VendingMachine_InService_Idle (initial), VendingMachine_InService_Paying
//! ```
#![allow(non_camel_case_types)]

mod triggers;
mod vending_machine;
mod vending_machine_in_service;
mod vending_machine_in_service_idle;
mod vending_machine_in_service_paying;
mod vending_machine_sold_out;

use statechart::prelude::*;
use triggers::{Coin, Select};
use vending_machine::VendingMachine;

fn main() {
    let mut chart = Statechart::new(VendingMachine::new(150, 2));
    println!("{chart:?}");

    // Idle does not handle Select.
    println!("Select -> {:?}", chart.trigger(Select));
    println!("Coin(100) -> credit {:?}", chart.trigger(Coin(100)));
    println!("Coin(20) -> credit {:?}", chart.trigger(Coin(20)));
    println!("Select -> {:?}", chart.trigger(Select));
    println!("Coin(50) -> credit {:?}", chart.trigger(Coin(50)));
    println!("Select -> {:?}", chart.trigger(Select));
    println!("  {chart:?}");

    println!("Coin(200) -> credit {:?}", chart.trigger(Coin(200)));
    println!("Select -> {:?}", chart.trigger(Select));
    println!("  {chart:?}");
    // Sold out: nobody takes the coin.
    println!("Coin(100) -> {:?}", chart.trigger(Coin(100)));
}
