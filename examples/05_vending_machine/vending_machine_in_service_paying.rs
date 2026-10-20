use crate::triggers::{Coin, Select, Vend};
use crate::vending_machine::VendingMachine;
use crate::vending_machine_in_service::VendingMachine_InService;
use crate::vending_machine_in_service_idle::VendingMachine_InService_Idle;
use crate::vending_machine_sold_out::VendingMachine_SoldOut;
use statechart::prelude::*;

/// Some credit is in; more coins add to it, `Select` buys an item.
#[derive(Debug, Default)]
pub struct VendingMachine_InService_Paying;

impl State for VendingMachine_InService_Paying {
    type Parent = VendingMachine_InService;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Coin>().trigger::<Select>();
    }
}

impl Trigger<Coin> for VendingMachine_InService_Paying {
    fn handle(&mut self, coin: &Coin, ctx: &mut Context<Self>) -> u32 {
        ctx.parent_mut().credit += coin.0;
        ctx.parent().credit
    }
}

impl Trigger<Select> for VendingMachine_InService_Paying {
    fn handle(&mut self, _: &Select, ctx: &mut Context<Self>) -> Vend {
        // The root is two levels up: look it up by type. It is always an ancestor, so the
        // `Option` is always `Some`.
        let price = ctx
            .ancestor::<VendingMachine>()
            .expect("the root is an ancestor")
            .price;
        let credit = ctx.parent().credit;
        if credit < price {
            return Vend::NeedMore {
                missing: price - credit,
            };
        }

        ctx.parent_mut().credit = 0;
        // One `&mut` ancestor at a time: read what is needed into locals first.
        let machine = ctx
            .ancestor_mut::<VendingMachine>()
            .expect("the root is an ancestor");
        machine.stock -= 1;
        let sold_out = machine.stock == 0;
        if sold_out {
            // Leaves InService (and its credit) behind.
            ctx.transition(VendingMachine_SoldOut);
        } else {
            ctx.transition(VendingMachine_InService_Idle);
        }
        Vend::Dispensed {
            change: credit - price,
        }
    }
}
