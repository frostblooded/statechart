// Runs the state code from the `04_door` example: `on_enter` / `on_exit` hooks writing to the
// root's log, and a state entered with data given by the transition (`child_explicit`).
#![allow(non_camel_case_types)]

#[path = "../examples/04_door/door.rs"]
mod door;
#[path = "../examples/04_door/door_closed.rs"]
mod door_closed;
#[path = "../examples/04_door/door_locked.rs"]
mod door_locked;
#[path = "../examples/04_door/door_open.rs"]
mod door_open;
#[path = "../examples/04_door/triggers.rs"]
mod triggers;

use door::Door;
use door_closed::Door_Closed;
use door_locked::Door_Locked;
use door_open::Door_Open;
use statechart::prelude::*;
use triggers::{Close, Lock, Open, Unlock};

#[test]
fn test_starts_closed_with_an_empty_log() {
    let chart = Statechart::new(Door::default());
    assert_eq!(chart.get_debug_name(), "Door -> Door_Closed");
    assert!(chart.root().log.is_empty());
}

#[test]
fn test_open_and_close_run_the_hooks() {
    let mut chart = Statechart::new(Door::default());
    chart.trigger(Open).unwrap();
    assert!(chart.is_active::<Door_Open>());
    assert_eq!(chart.root().log, ["door swings open"]);
    chart.trigger(Close).unwrap();
    assert!(chart.is_active::<Door_Closed>());
    assert_eq!(chart.root().log, ["door swings open", "door swings shut"]);
}

#[test]
fn test_an_open_door_cannot_be_opened_or_locked() {
    let mut chart = Statechart::new(Door::default());
    chart.trigger(Open).unwrap();
    assert_eq!(chart.trigger(Open), Err(NotHandled));
    assert_eq!(chart.trigger(Lock { code: 1234 }), Err(NotHandled));
}

#[test]
fn test_lock_keeps_the_code_and_only_that_code_unlocks() {
    let mut chart = Statechart::new(Door::default());
    chart.trigger(Lock { code: 1234 }).unwrap();
    assert_eq!(chart.state::<Door_Locked>().unwrap().code, 1234);
    assert_eq!(chart.root().log, ["bolt slides in"]);
    assert_eq!(chart.trigger(Open), Err(NotHandled));

    assert_eq!(chart.trigger(Unlock { code: 1 }), Ok(false));
    assert!(chart.is_active::<Door_Locked>());
    assert_eq!(chart.trigger(Unlock { code: 1234 }), Ok(true));
    assert!(chart.is_active::<Door_Closed>());
    assert_eq!(chart.root().log, ["bolt slides in", "bolt slides out"]);
}
