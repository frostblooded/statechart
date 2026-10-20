// Runs the state code from the `05_vending_machine` example: a nested state reading and
// writing its parent's data (`parent_mut`) and the root's (`ancestor` / `ancestor_mut`).
#![allow(non_camel_case_types)]

#[path = "../examples/05_vending_machine/triggers.rs"]
mod triggers;
#[path = "../examples/05_vending_machine/vending_machine.rs"]
mod vending_machine;
#[path = "../examples/05_vending_machine/vending_machine_in_service.rs"]
mod vending_machine_in_service;
#[path = "../examples/05_vending_machine/vending_machine_in_service_idle.rs"]
mod vending_machine_in_service_idle;
#[path = "../examples/05_vending_machine/vending_machine_in_service_paying.rs"]
mod vending_machine_in_service_paying;
#[path = "../examples/05_vending_machine/vending_machine_sold_out.rs"]
mod vending_machine_sold_out;

use statechart::prelude::*;
use triggers::{Coin, Select, Vend};
use vending_machine::VendingMachine;
use vending_machine_in_service::VendingMachine_InService;
use vending_machine_in_service_idle::VendingMachine_InService_Idle;
use vending_machine_in_service_paying::VendingMachine_InService_Paying;
use vending_machine_sold_out::VendingMachine_SoldOut;

#[test]
fn test_starts_idle_with_no_credit() {
    let chart = Statechart::new(VendingMachine::new(150, 2));
    assert_eq!(
        chart.get_debug_name(),
        "VendingMachine { price: 150, stock: 2 } -> VendingMachine_InService { credit: 0 } \
         -> VendingMachine_InService_Idle"
    );
}

#[test]
fn test_coins_add_up_in_the_parent() {
    let mut chart = Statechart::new(VendingMachine::new(150, 2));
    assert_eq!(chart.trigger(Coin(100)), Ok(100));
    assert!(chart.is_active::<VendingMachine_InService_Paying>());
    assert_eq!(chart.trigger(Coin(20)), Ok(120));
    assert_eq!(
        chart.state::<VendingMachine_InService>().unwrap().credit,
        120
    );
}

#[test]
fn test_selecting_without_paying_is_not_handled() {
    let mut chart = Statechart::new(VendingMachine::new(150, 2));
    assert_eq!(chart.trigger(Select), Err(NotHandled));
}

#[test]
fn test_select_asks_for_the_missing_amount() {
    let mut chart = Statechart::new(VendingMachine::new(150, 2));
    chart.trigger(Coin(100)).unwrap();
    assert_eq!(chart.trigger(Select), Ok(Vend::NeedMore { missing: 50 }));
    assert!(chart.is_active::<VendingMachine_InService_Paying>());
}

#[test]
fn test_select_dispenses_gives_change_and_takes_one_from_stock() {
    let mut chart = Statechart::new(VendingMachine::new(150, 2));
    chart.trigger(Coin(100)).unwrap();
    chart.trigger(Coin(100)).unwrap();
    assert_eq!(chart.trigger(Select), Ok(Vend::Dispensed { change: 50 }));
    assert!(chart.is_active::<VendingMachine_InService_Idle>());
    assert_eq!(chart.state::<VendingMachine_InService>().unwrap().credit, 0);
    assert_eq!(chart.root().stock, 1);
}

#[test]
fn test_the_last_item_sells_the_machine_out() {
    let mut chart = Statechart::new(VendingMachine::new(150, 1));
    chart.trigger(Coin(150)).unwrap();
    assert_eq!(chart.trigger(Select), Ok(Vend::Dispensed { change: 0 }));
    assert_eq!(
        chart.get_debug_name(),
        "VendingMachine { price: 150, stock: 0 } -> VendingMachine_SoldOut"
    );
    assert!(chart.is_active::<VendingMachine_SoldOut>());
    // Nobody takes the coin: it falls back out of the slot.
    assert_eq!(chart.trigger(Coin(100)), Err(NotHandled));
}
