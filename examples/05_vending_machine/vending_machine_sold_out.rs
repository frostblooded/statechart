use crate::vending_machine::VendingMachine;
use statechart::prelude::*;

/// Empty. Handles nothing, so every trigger answers `Err(NotHandled)`.
#[derive(Debug, Default)]
pub struct VendingMachine_SoldOut;

impl State for VendingMachine_SoldOut {
    type Parent = VendingMachine;
}
