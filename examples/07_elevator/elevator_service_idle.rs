use crate::elevator::Elevator;
use crate::elevator_service::Elevator_Service;
use crate::elevator_service_doors_open::Elevator_Service_DoorsOpen;
use crate::elevator_service_moving_down::Elevator_Service_Moving_Down;
use crate::elevator_service_moving_up::Elevator_Service_Moving_Up;
use statechart::prelude::*;
use std::cmp::Ordering;

/// Waiting for a call.
#[derive(Debug, Default)]
pub struct Elevator_Service_Idle;

impl State for Elevator_Service_Idle {
    type Parent = Elevator_Service;

    fn update(&mut self, ctx: &mut Context<Self>) {
        let Some(&next) = ctx.parent().queue.front() else {
            return;
        };
        let floor = ctx.ancestor::<Elevator>().expect("the root").floor;
        // Moving is not active: these transitions build it with its `child_with` constructor,
        // which reads the queue, so its target is `next`.
        match next.cmp(&floor) {
            Ordering::Greater => ctx.transition(Elevator_Service_Moving_Up),
            Ordering::Less => ctx.transition(Elevator_Service_Moving_Down),
            Ordering::Equal => ctx.transition(Elevator_Service_DoorsOpen::default()),
        }
    }
}
