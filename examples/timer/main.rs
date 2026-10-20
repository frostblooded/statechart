//! `cargo run --example timer`: a 5-second countdown that pauses for two seconds, then elapses
//! and resets. Prints the active chain every second, forever (Ctrl-C to stop).
#![allow(non_camel_case_types)]

mod timer;
mod timer_active;
mod timer_active_paused;
mod timer_active_running;
mod timer_elapsed;
mod triggers;

use statechart::prelude::*;
use std::thread;
use std::time::Duration;
use timer::Timer;
use timer_elapsed::Timer_Elapsed;
use triggers::{GetRemaining, Pause, Reset, Resume};

fn main() {
    let mut chart = Statechart::new(Timer::new(Duration::from_secs(5)));
    println!("{chart:?}");

    let mut second = 0;
    loop {
        thread::sleep(Duration::from_secs(1));
        second += 1;
        match second {
            2 => chart.trigger(Pause).expect("a running timer can pause"),
            4 => chart.trigger(Resume).expect("a paused timer can resume"),
            _ => {}
        }
        chart.update();
        match chart.trigger(GetRemaining) {
            Ok(remaining) => println!("{chart:?} ({}s left)", remaining.as_secs_f32().ceil()),
            Err(NotHandled) => println!("{chart:?}"),
        }
        if chart.is_active::<Timer_Elapsed>() {
            chart.trigger(Reset).expect("the root handles Reset");
            second = 0;
        }
    }
}
