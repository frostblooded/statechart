//! The triggers the vending machine reacts to.

use statechart::trigger::Event;

/// Inserts a coin of the given value. Answers the credit after it.
pub struct Coin(pub u32);
impl Event for Coin {
    type Output = u32;
}

/// Asks for an item. Only handled once some credit is in.
pub struct Select;
impl Event for Select {
    type Output = Vend;
}

/// The answer to `Select`.
#[derive(Debug, PartialEq, Eq)]
pub enum Vend {
    /// An item dropped; `change` comes back.
    Dispensed { change: u32 },
    /// Not enough credit yet.
    NeedMore { missing: u32 },
}
