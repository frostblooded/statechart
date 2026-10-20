use std::fmt::{Debug, Formatter};
use std::time::Duration;
use statechart::statechart::Statechart;
use crate::timer_root::Timer_Root;

pub struct Timer {
    statechart: Statechart,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Timer {
            statechart: Statechart::new(Box::new(Timer_Root::new(duration))),
        }
    }

    pub fn update(&mut self) {
        self.statechart.update();
    }
}

impl Debug for Timer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.statechart)
    }
}
