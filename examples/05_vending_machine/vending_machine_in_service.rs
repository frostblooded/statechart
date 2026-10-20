use crate::vending_machine::VendingMachine;
use crate::vending_machine_in_service_idle::VendingMachine_InService_Idle;
use crate::vending_machine_in_service_paying::VendingMachine_InService_Paying;
use statechart::prelude::*;

/// Selling. A composite state: its `credit` is shared by its children, which reach it with
/// `ctx.parent()` / `ctx.parent_mut()`.
#[derive(Debug, Default)]
pub struct VendingMachine_InService {
    pub credit: u32,
}

impl State for VendingMachine_InService {
    type Parent = VendingMachine;

    fn define(def: &mut Definition<Self>) {
        def.region::<VendingMachine_InService_Idle>()
            .child::<VendingMachine_InService_Paying>();
    }
}
