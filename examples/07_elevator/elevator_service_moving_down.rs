use crate::elevator::Elevator;
use crate::elevator_service_doors_open::Elevator_Service_DoorsOpen;
use crate::elevator_service_moving::Elevator_Service_Moving;
use statechart::prelude::*;

/// Going down, one floor per update.
#[derive(Debug, Default)]
pub struct Elevator_Service_Moving_Down;

impl State for Elevator_Service_Moving_Down {
    type Parent = Elevator_Service_Moving;

    fn update(&mut self, ctx: &mut Context<Self>) {
        let target = ctx.parent().target;
        // The root is three levels up; `ancestor_mut` finds it by type.
        let elevator = ctx.ancestor_mut::<Elevator>().expect("the root");
        elevator.floor -= 1;
        if elevator.floor == target {
            ctx.transition(Elevator_Service_DoorsOpen::default());
        }
    }
}
