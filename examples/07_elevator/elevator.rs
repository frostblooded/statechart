use crate::elevator_maintenance::Elevator_Maintenance;
use crate::elevator_service::Elevator_Service;
use crate::triggers::StartMaintenance;
use statechart::prelude::*;

/// The root: the car's `floor`, between 0 and `top_floor`.
#[derive(Debug)]
pub struct Elevator {
    pub floor: i32,
    pub top_floor: i32,
}

impl Elevator {
    pub fn new(top_floor: i32) -> Self {
        Elevator {
            floor: 0,
            top_floor,
        }
    }
}

impl State for Elevator {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<Elevator_Service>()
            .child::<Elevator_Maintenance>();
        def.trigger::<StartMaintenance>();
    }
}

impl Trigger<StartMaintenance> for Elevator {
    // The root handles it, so it works from every state, however deep.
    fn handle(&mut self, _: &StartMaintenance, ctx: &mut Context<Self>) {
        ctx.transition(Elevator_Maintenance);
    }
}
