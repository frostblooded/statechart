use crate::timer::Timer;
use std::thread;
use std::time::Duration;

mod timer;
mod timer_elapsed;
mod timer_root;
mod timer_running;

fn main() {
    let mut timer: Timer = Timer::new(Duration::from_secs(3));
    println!("Timer statechart: {:?}", timer);

    loop {
        thread::sleep(Duration::from_secs(1));
        timer.update();
        println!("Timer statechart: {:?}", timer);
    }
}
