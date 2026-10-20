use crate::triggers::Coin;
use crate::vending_machine_in_service::VendingMachine_InService;
use crate::vending_machine_in_service_paying::VendingMachine_InService_Paying;
use statechart::prelude::*;

/// Waiting for the first coin.
#[derive(Debug, Default)]
pub struct VendingMachine_InService_Idle;

impl State for VendingMachine_InService_Idle {
    type Parent = VendingMachine_InService;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Coin>();
    }
}

impl Trigger<Coin> for VendingMachine_InService_Idle {
    fn handle(&mut self, coin: &Coin, ctx: &mut Context<Self>) -> u32 {
        // `parent_mut()` is typed: it is a `&mut VendingMachine_InService`, no lookup needed.
        let service = ctx.parent_mut();
        service.credit += coin.0;
        let credit = service.credit;
        ctx.transition(VendingMachine_InService_Paying);
        credit
    }
}
