use std::thread;
use std::time::Duration;
use crate::timer::Timer;

mod timer;
mod timer_running;
mod timer_elapsed;
mod timer_root;

fn main() {
    let mut timer: Timer = Timer::new(Duration::from_secs(3));
    println!("Timer statechart: {:?}", timer);

    loop {
        thread::sleep(Duration::from_secs(1));
        timer.update();
        println!("Timer statechart: {:?}", timer);
    }
}
