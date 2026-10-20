use crate::elevator::Elevator;
use crate::elevator_service_doors_open::Elevator_Service_DoorsOpen;
use crate::elevator_service_idle::Elevator_Service_Idle;
use crate::elevator_service_moving::Elevator_Service_Moving;
use crate::triggers::Call;
use statechart::prelude::*;
use std::collections::VecDeque;

/// In service: serves the calls in `queue`, oldest first.
#[derive(Debug, Default)]
pub struct Elevator_Service {
    pub queue: VecDeque<i32>,
}

impl State for Elevator_Service {
    type Parent = Elevator;

    fn define(def: &mut Definition<Self>) {
        def.region::<Elevator_Service_Idle>()
            // Whenever the library has to build Moving (a transition to Moving_Up or
            // Moving_Down while Moving is inactive), it heads for the oldest call, or to the
            // ground floor if there is none.
            .child_with(|service: &Elevator_Service| Elevator_Service_Moving {
                target: service.queue.front().copied().unwrap_or(0),
            })
            .child::<Elevator_Service_DoorsOpen>();
        // Handled here, so it works whether the car is idle, moving or has its doors open.
        def.trigger::<Call>();
    }
}

impl Trigger<Call> for Elevator_Service {
    /// Whether the floor exists.
    fn handle(&mut self, call: &Call, ctx: &mut Context<Self>) -> bool {
        if !(0..=ctx.parent().top_floor).contains(&call.0) {
            return false;
        }
        if !self.queue.contains(&call.0) {
            self.queue.push_back(call.0);
        }
        true
    }
}
