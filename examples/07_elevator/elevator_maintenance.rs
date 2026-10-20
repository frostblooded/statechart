use crate::elevator::Elevator;
use crate::elevator_service_idle::Elevator_Service_Idle;
use crate::elevator_service_moving_down::Elevator_Service_Moving_Down;
use crate::triggers::Resume;
use statechart::prelude::*;

/// Out of service: takes no calls. The car stays where it stopped.
#[derive(Debug, Default)]
pub struct Elevator_Maintenance;

impl State for Elevator_Maintenance {
    type Parent = Elevator;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Resume>();
    }
}

impl Trigger<Resume> for Elevator_Maintenance {
    fn handle(&mut self, _: &Resume, ctx: &mut Context<Self>) {
        if ctx.parent().floor == 0 {
            ctx.transition(Elevator_Service_Idle);
        } else {
            // Two levels below the inactive Elevator_Service: the transition builds Service
            // (`Default`, an empty queue) and Moving (its constructor, which heads to floor 0
            // when the queue is empty) before it enters Moving_Down.
            ctx.transition(Elevator_Service_Moving_Down);
        }
    }
}
