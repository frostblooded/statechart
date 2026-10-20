use crate::traffic_light::TrafficLight;
use crate::traffic_light_green::TrafficLight_Green;
use statechart::prelude::*;

/// How many updates the light stays red.
pub const DURATION: u32 = 3;

/// The light is red. `ticks` is this state's own data: it starts at 0 each time the state is
/// entered and is dropped when the state exits.
#[derive(Debug, Default)]
pub struct TrafficLight_Red {
    pub ticks: u32,
}

impl State for TrafficLight_Red {
    type Parent = TrafficLight;

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == DURATION {
            ctx.transition(TrafficLight_Green::default());
        }
    }
}
