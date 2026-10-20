use crate::vending_machine_in_service::VendingMachine_InService;
use crate::vending_machine_sold_out::VendingMachine_SoldOut;
use statechart::prelude::*;

/// The root: what one item costs and how many are left. Its descendants read and change this
/// data.
#[derive(Debug)]
pub struct VendingMachine {
    pub price: u32,
    pub stock: u32,
}

impl VendingMachine {
    pub fn new(price: u32, stock: u32) -> Self {
        VendingMachine { price, stock }
    }
}

impl State for VendingMachine {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<VendingMachine_InService>()
            .child::<VendingMachine_SoldOut>();
    }
}
