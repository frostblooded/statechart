// Runs the state code from the `07_elevator` example: four levels of nesting, `ancestor_mut`
// from a deep state, a child built from its parent's data (`child_with`),
// and transitions into states whose ancestors are not active yet.
#![allow(non_camel_case_types)]

#[path = "../examples/07_elevator/elevator.rs"]
mod elevator;
#[path = "../examples/07_elevator/elevator_maintenance.rs"]
mod elevator_maintenance;
#[path = "../examples/07_elevator/elevator_service.rs"]
mod elevator_service;
#[path = "../examples/07_elevator/elevator_service_doors_open.rs"]
mod elevator_service_doors_open;
#[path = "../examples/07_elevator/elevator_service_idle.rs"]
mod elevator_service_idle;
#[path = "../examples/07_elevator/elevator_service_moving.rs"]
mod elevator_service_moving;
#[path = "../examples/07_elevator/elevator_service_moving_down.rs"]
mod elevator_service_moving_down;
#[path = "../examples/07_elevator/elevator_service_moving_up.rs"]
mod elevator_service_moving_up;
#[path = "../examples/07_elevator/triggers.rs"]
mod triggers;

use elevator::Elevator;
use elevator_maintenance::Elevator_Maintenance;
use elevator_service::Elevator_Service;
use elevator_service_doors_open::Elevator_Service_DoorsOpen;
use elevator_service_idle::Elevator_Service_Idle;
use elevator_service_moving::Elevator_Service_Moving;
use elevator_service_moving_down::Elevator_Service_Moving_Down;
use elevator_service_moving_up::Elevator_Service_Moving_Up;
use statechart::prelude::*;
use triggers::{Call, Resume, StartMaintenance};

fn updates(chart: &mut Statechart<Elevator>, n: u32) {
    for _ in 0..n {
        chart.update();
    }
}

#[test]
fn test_starts_idle_on_the_ground_floor() {
    let chart = Statechart::new(Elevator::new(5));
    assert_eq!(
        chart.get_debug_name(),
        "Elevator { floor: 0, top_floor: 5 } -> Elevator_Service { queue: [] } \
         -> Elevator_Service_Idle"
    );
}

#[test]
fn test_calls_outside_the_building_are_refused() {
    let mut chart = Statechart::new(Elevator::new(5));
    assert_eq!(chart.trigger(Call(6)), Ok(false));
    assert_eq!(chart.trigger(Call(-1)), Ok(false));
    assert_eq!(chart.trigger(Call(3)), Ok(true));
    // Calling a floor twice queues it once.
    assert_eq!(chart.trigger(Call(3)), Ok(true));
    assert_eq!(chart.state::<Elevator_Service>().unwrap().queue, [3]);
}

#[test]
fn test_a_call_moves_up_floor_by_floor_and_opens_the_doors() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(Call(2)).unwrap();
    chart.update();
    // Idle jumped straight to Moving_Up; Moving was built from Service's queue.
    assert_eq!(
        chart.get_debug_name(),
        "Elevator { floor: 0, top_floor: 5 } -> Elevator_Service { queue: [2] } \
         -> Elevator_Service_Moving { target: 2 } -> Elevator_Service_Moving_Up"
    );
    chart.update();
    assert_eq!(chart.root().floor, 1);
    assert!(chart.is_active::<Elevator_Service_Moving_Up>());
    chart.update();
    assert_eq!(chart.root().floor, 2);
    // Arrived: the doors open and the call leaves the queue.
    assert!(chart.is_active::<Elevator_Service_DoorsOpen>());
    assert!(chart.state::<Elevator_Service>().unwrap().queue.is_empty());
    updates(&mut chart, elevator_service_doors_open::OPEN_TICKS);
    assert!(chart.is_active::<Elevator_Service_Idle>());
}

#[test]
fn test_calls_are_served_in_order_up_and_down() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(Call(1)).unwrap();
    chart.update();
    chart.trigger(Call(0)).unwrap();
    chart.update();
    assert!(chart.is_active::<Elevator_Service_DoorsOpen>());
    assert_eq!(
        chart.state::<Elevator_Service_DoorsOpen>().unwrap().ticks,
        0
    );
    updates(&mut chart, elevator_service_doors_open::OPEN_TICKS);
    chart.update();
    assert_eq!(chart.state::<Elevator_Service_Moving>().unwrap().target, 0);
    assert!(chart.is_active::<Elevator_Service_Moving_Down>());
    chart.update();
    assert_eq!(chart.root().floor, 0);
    assert!(chart.is_active::<Elevator_Service_DoorsOpen>());
}

#[test]
fn test_a_call_for_the_current_floor_opens_the_doors() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(Call(0)).unwrap();
    chart.update();
    assert!(chart.is_active::<Elevator_Service_DoorsOpen>());
}

#[test]
fn test_maintenance_from_anywhere_drops_the_calls() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(Call(3)).unwrap();
    updates(&mut chart, 3);
    assert_eq!(chart.root().floor, 2);
    chart.trigger(StartMaintenance).unwrap();
    assert_eq!(
        chart.get_debug_name(),
        "Elevator { floor: 2, top_floor: 5 } -> Elevator_Maintenance"
    );
    assert!(chart.is_active::<Elevator_Maintenance>());
    // Nobody takes calls during maintenance.
    assert_eq!(chart.trigger(Call(1)), Err(NotHandled));
}

#[test]
fn test_resume_goes_straight_down_to_the_ground_floor() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(Call(2)).unwrap();
    updates(&mut chart, 3);
    chart.trigger(StartMaintenance).unwrap();
    chart.trigger(Resume).unwrap();
    // Service and Moving were both inactive: built by their constructors on the way down.
    assert_eq!(
        chart.get_debug_name(),
        "Elevator { floor: 2, top_floor: 5 } -> Elevator_Service { queue: [] } \
         -> Elevator_Service_Moving { target: 0 } -> Elevator_Service_Moving_Down"
    );
    updates(&mut chart, 2);
    assert_eq!(chart.root().floor, 0);
    assert!(chart.is_active::<Elevator_Service_DoorsOpen>());
}

#[test]
fn test_resume_on_the_ground_floor_is_idle() {
    let mut chart = Statechart::new(Elevator::new(5));
    chart.trigger(StartMaintenance).unwrap();
    chart.trigger(Resume).unwrap();
    assert!(chart.is_active::<Elevator_Service_Idle>());
    // Resume only means something during maintenance.
    assert_eq!(chart.trigger(Resume), Err(NotHandled));
}
