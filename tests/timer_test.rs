// Runs the state code from the `timer` example, so the example stays in sync with the public API.
#![allow(non_camel_case_types)]

#[path = "../examples/timer/timer.rs"]
mod timer;
#[path = "../examples/timer/timer_active.rs"]
mod timer_active;
#[path = "../examples/timer/timer_active_paused.rs"]
mod timer_active_paused;
#[path = "../examples/timer/timer_active_running.rs"]
mod timer_active_running;
#[path = "../examples/timer/timer_elapsed.rs"]
mod timer_elapsed;
#[path = "../examples/timer/triggers.rs"]
mod triggers;

use statechart::prelude::*;
use std::any::TypeId;
use std::thread;
use std::time::Duration;
use timer::Timer;
use timer_active::Timer_Active;
use timer_active_paused::Timer_Active_Paused;
use timer_active_running::Timer_Active_Running;
use timer_elapsed::Timer_Elapsed;
use triggers::{GetRemaining, Pause, Reset, Resume};

const LONG: Duration = Duration::from_secs(60);

#[test]
fn test_timer_starts_running_with_the_full_duration() {
    let mut statechart = Statechart::new(Timer::new(LONG));
    assert_eq!(
        statechart.get_debug_name(),
        "Timer { duration: 60s } -> Timer_Active { remaining: 60.0s } -> Timer_Active_Running"
    );
    assert_eq!(
        statechart.get_recursive_active_type_ids(),
        vec![
            TypeId::of::<Timer_Active_Running>(),
            TypeId::of::<Timer_Active>()
        ]
    );
    let remaining = statechart.trigger(GetRemaining).unwrap();
    assert!(remaining <= LONG && remaining > LONG - Duration::from_secs(1));
}

#[test]
fn test_timer_stays_running_before_duration() {
    let mut statechart = Statechart::new(Timer::new(LONG));
    statechart.update();
    assert!(statechart.is_active::<Timer_Active_Running>());
    assert!(statechart.state::<Timer_Active>().unwrap().remaining < LONG);
}

#[test]
fn test_timer_transitions_to_elapsed_after_duration() {
    let mut statechart = Statechart::new(Timer::new(Duration::from_millis(10)));
    thread::sleep(Duration::from_millis(20));
    statechart.update();
    assert_eq!(
        statechart.get_recursive_active_type_ids(),
        vec![TypeId::of::<Timer_Elapsed>()]
    );
    assert_eq!(
        statechart.get_debug_name(),
        "Timer { duration: 10ms } -> Timer_Elapsed"
    );
    // Neither Timer_Elapsed nor Timer handles GetRemaining.
    assert_eq!(statechart.trigger(GetRemaining), Err(NotHandled));
}

#[test]
fn test_timer_with_zero_duration_elapses_on_the_first_update() {
    let mut statechart = Statechart::new(Timer::new(Duration::ZERO));
    assert!(statechart.get_debug_name().starts_with("Timer"));
    statechart.update();
    assert!(statechart.get_debug_name().ends_with("-> Timer_Elapsed"));
}

#[test]
fn test_pause_freezes_the_remaining_time_until_resume() {
    let mut statechart = Statechart::new(Timer::new(LONG));
    statechart.trigger(Pause).unwrap();
    assert!(statechart.is_active::<Timer_Active_Paused>());
    // Paused does not handle GetRemaining: Timer_Active answers with the frozen value.
    let frozen = statechart.trigger(GetRemaining).unwrap();
    thread::sleep(Duration::from_millis(20));
    statechart.update();
    assert_eq!(statechart.trigger(GetRemaining), Ok(frozen));
    // Paused does not handle Pause, and neither do its ancestors.
    assert_eq!(statechart.trigger(Pause), Err(NotHandled));

    statechart.trigger(Resume).unwrap();
    assert!(statechart.is_active::<Timer_Active_Running>());
    assert_eq!(statechart.trigger(Resume), Err(NotHandled));
    thread::sleep(Duration::from_millis(20));
    statechart.update();
    assert!(statechart.trigger(GetRemaining).unwrap() < frozen);
}

#[test]
fn test_reset_restarts_the_countdown_from_any_state() {
    let mut statechart = Statechart::new(Timer::new(Duration::from_millis(10)));
    thread::sleep(Duration::from_millis(20));
    statechart.update();
    assert!(statechart.is_active::<Timer_Elapsed>());

    // From Elapsed.
    statechart.trigger(Reset).unwrap();
    assert!(statechart.is_active::<Timer_Active_Running>());
    assert_eq!(
        statechart.state::<Timer_Active>().unwrap().remaining,
        Duration::from_millis(10)
    );

    // From Paused.
    statechart.trigger(Pause).unwrap();
    statechart.trigger(Reset).unwrap();
    assert!(statechart.is_active::<Timer_Active_Running>());
    assert_eq!(
        statechart.state::<Timer_Active>().unwrap().remaining,
        Duration::from_millis(10)
    );
}
