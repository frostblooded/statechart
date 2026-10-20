// Runs the state code from the `timer` example, so the example stays in sync with the public API.
#![allow(non_camel_case_types)]

#[path = "../examples/timer/timer_elapsed.rs"]
mod timer_elapsed;
#[path = "../examples/timer/timer_root.rs"]
mod timer_root;
#[path = "../examples/timer/timer_running.rs"]
mod timer_running;

use std::any::TypeId;
use std::thread;
use std::time::Duration;
use statechart::statechart::Statechart;
use timer_elapsed::Timer_Elapsed;
use timer_root::Timer_Root;
use timer_running::Timer_Running;

#[test]
fn test_timer_stays_running_before_duration() {
    let mut statechart = Statechart::new(Box::new(Timer_Root::new(Duration::from_secs(60))));
    statechart.update();
    assert_eq!(statechart.get_recursive_active_type_ids(), vec![TypeId::of::<Timer_Running>()]);
}

#[test]
fn test_timer_transitions_to_elapsed_after_duration() {
    let mut statechart = Statechart::new(Box::new(Timer_Root::new(Duration::from_millis(10))));
    assert_eq!(statechart.get_recursive_active_type_ids(), vec![TypeId::of::<Timer_Running>()]);
    thread::sleep(Duration::from_millis(20));
    statechart.update();
    assert_eq!(statechart.get_recursive_active_type_ids(), vec![TypeId::of::<Timer_Elapsed>()]);
}

#[test]
fn test_timer_debug_name() {
    let mut statechart = Statechart::new(Box::new(Timer_Root::new(Duration::ZERO)));
    assert!(statechart.get_debug_name().starts_with("Timer_Root"));
    statechart.update();
    assert!(statechart.get_debug_name().ends_with("-> Timer_Elapsed"));
}
