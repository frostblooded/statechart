use crate::traffic_light::TrafficLight;
use crate::traffic_light_red::TrafficLight_Red;
use statechart::prelude::*;

/// How many updates the light stays yellow.
pub const DURATION: u32 = 1;

/// The light is yellow. `ticks` is this state's own data: it starts at 0 each time the state is
/// entered and is dropped when the state exits.
#[derive(Debug, Default)]
pub struct TrafficLight_Yellow {
    pub ticks: u32,
}

impl State for TrafficLight_Yellow {
    type Parent = TrafficLight;

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == DURATION {
            ctx.transition(TrafficLight_Red::default());
        }
    }
}
