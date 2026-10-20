// Runs the state code from the `08_washing_machine` example: a state with two parallel
// regions (the wash cycle and the thermostat) that run side by side.
#![allow(non_camel_case_types)]

#[path = "../examples/08_washing_machine/triggers.rs"]
mod triggers;
#[path = "../examples/08_washing_machine/washing_machine.rs"]
mod washing_machine;
#[path = "../examples/08_washing_machine/washing_machine_done.rs"]
mod washing_machine_done;
#[path = "../examples/08_washing_machine/washing_machine_ready.rs"]
mod washing_machine_ready;
#[path = "../examples/08_washing_machine/washing_machine_running.rs"]
mod washing_machine_running;
#[path = "../examples/08_washing_machine/washing_machine_running_fill.rs"]
mod washing_machine_running_fill;
#[path = "../examples/08_washing_machine/washing_machine_running_heating.rs"]
mod washing_machine_running_heating;
#[path = "../examples/08_washing_machine/washing_machine_running_hot.rs"]
mod washing_machine_running_hot;
#[path = "../examples/08_washing_machine/washing_machine_running_rinse.rs"]
mod washing_machine_running_rinse;
#[path = "../examples/08_washing_machine/washing_machine_running_spin.rs"]
mod washing_machine_running_spin;
#[path = "../examples/08_washing_machine/washing_machine_running_wash.rs"]
mod washing_machine_running_wash;

use statechart::prelude::*;
use triggers::{Cancel, Start, Unload};
use washing_machine::WashingMachine;
use washing_machine_done::WashingMachine_Done;
use washing_machine_ready::WashingMachine_Ready;
use washing_machine_running::WashingMachine_Running;
use washing_machine_running_fill::WashingMachine_Running_Fill;
use washing_machine_running_heating::WashingMachine_Running_Heating;
use washing_machine_running_hot::WashingMachine_Running_Hot;
use washing_machine_running_rinse::WashingMachine_Running_Rinse;
use washing_machine_running_spin::WashingMachine_Running_Spin;
use washing_machine_running_wash::WashingMachine_Running_Wash;

fn started() -> Statechart<WashingMachine> {
    let mut chart = Statechart::new(WashingMachine);
    chart.trigger(Start).unwrap();
    chart
}

fn temperature(chart: &Statechart<WashingMachine>) -> u32 {
    chart.state::<WashingMachine_Running>().unwrap().temperature
}

#[test]
fn test_start_enters_both_regions() {
    let mut chart = Statechart::new(WashingMachine);
    assert_eq!(
        chart.get_debug_name(),
        "WashingMachine -> WashingMachine_Ready"
    );
    chart.trigger(Start).unwrap();
    assert_eq!(
        chart.get_debug_name(),
        "WashingMachine -> WashingMachine_Running { temperature: 20 } \
         -> [WashingMachine_Running_Fill { ticks: 0 }, WashingMachine_Running_Heating]"
    );
}

#[test]
fn test_the_cycle_moves_through_its_phases() {
    let mut chart = started();
    let phases = [
        washing_machine_running_fill::DURATION,
        washing_machine_running_wash::DURATION,
        washing_machine_running_rinse::DURATION,
    ];
    let checks: [fn(&Statechart<WashingMachine>) -> bool; 3] = [
        |c| c.is_active::<WashingMachine_Running_Wash>(),
        |c| c.is_active::<WashingMachine_Running_Rinse>(),
        |c| c.is_active::<WashingMachine_Running_Spin>(),
    ];
    for (duration, check) in phases.into_iter().zip(checks) {
        for _ in 0..duration {
            chart.update();
        }
        assert!(check(&chart));
    }
    for _ in 0..washing_machine_running_spin::DURATION {
        chart.update();
    }
    // The whole Running state, both regions, is gone.
    assert_eq!(chart.get_debug_name(), "WashingMachine -> WashingMachine_Done");
}

#[test]
fn test_the_thermostat_runs_on_its_own_beside_the_cycle() {
    let mut chart = started();
    chart.update();
    assert_eq!(temperature(&chart), 30);
    assert!(chart.is_active::<WashingMachine_Running_Heating>());
    chart.update();
    assert_eq!(temperature(&chart), 40);
    assert!(chart.is_active::<WashingMachine_Running_Hot>());
    // Fill is over too, in the other region, after the same update.
    assert!(chart.is_active::<WashingMachine_Running_Wash>());
    chart.update();
    assert_eq!(temperature(&chart), 35);
    assert!(chart.is_active::<WashingMachine_Running_Hot>());
    chart.update();
    assert_eq!(temperature(&chart), 30);
    assert!(chart.is_active::<WashingMachine_Running_Heating>());
    // Still washing: the thermostat's transitions leave the cycle region alone.
    assert_eq!(
        chart.state::<WashingMachine_Running_Wash>().unwrap().ticks,
        2
    );
}

#[test]
fn test_cancel_stops_both_regions() {
    let mut chart = started();
    chart.update();
    chart.update();
    assert_eq!(chart.trigger(Start), Err(NotHandled));
    chart.trigger(Cancel).unwrap();
    assert!(chart.is_active::<WashingMachine_Ready>());
    assert!(!chart.is_active::<WashingMachine_Running_Wash>());
    assert!(!chart.is_active::<WashingMachine_Running_Hot>());
}

#[test]
fn test_unload_after_the_cycle() {
    let mut chart = started();
    while !chart.is_active::<WashingMachine_Done>() {
        chart.update();
    }
    assert_eq!(chart.trigger(Cancel), Err(NotHandled));
    assert_eq!(chart.trigger(Start), Err(NotHandled));
    chart.trigger(Unload).unwrap();
    assert!(chart.is_active::<WashingMachine_Ready>());
    // A new cycle starts cold.
    chart.trigger(Start).unwrap();
    assert_eq!(temperature(&chart), 20);
    assert!(chart.is_active::<WashingMachine_Running_Fill>());
}
