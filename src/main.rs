mod statechart_update_context;
mod timer_state_chart;
mod timer_meta_data;
mod state_behavior;
mod timer_running;
mod timer_elapsed;
mod timer;

use crate::timer::Timer;
use std::thread;
use std::time::Duration;
use timer_state_chart::TimerStateChart;

fn main() {
    let mut statechart: TimerStateChart = TimerStateChart::new();

    loop {
        thread::sleep(Duration::from_secs(1));
        statechart.update();
        let root: &Timer = &statechart.root;
        println!("Active states: {:?}", root.meta_data.active_children_idx)
    }
}
