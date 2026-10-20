use crate::elevator_service::Elevator_Service;
use crate::elevator_service_moving_down::Elevator_Service_Moving_Down;
use crate::elevator_service_moving_up::Elevator_Service_Moving_Up;
use statechart::prelude::*;

/// Travelling to `target`. Built by the constructor given in `Elevator_Service::define`.
#[derive(Debug)]
pub struct Elevator_Service_Moving {
    pub target: i32,
}

impl State for Elevator_Service_Moving {
    type Parent = Elevator_Service;

    fn define(def: &mut Definition<Self>) {
        def.region::<Elevator_Service_Moving_Up>()
            .child::<Elevator_Service_Moving_Down>();
    }
}
