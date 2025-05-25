mod statechart_update_context;
mod timer;
mod state_meta_data;
mod timer_running;
mod timer_elapsed;
mod timer_root;
mod state;
mod custom_state_trait;

use std::thread;
use std::time::Duration;
use timer::Timer;
use crate::state::State;

fn main() {
    let mut statechart: Timer = Timer::new();

    loop {
        thread::sleep(Duration::from_secs(1));
        statechart.update();
        let root: &State = &statechart.root;
        println!("Active states: {:?}", root.meta_data.active_children_idx)
    }
}
