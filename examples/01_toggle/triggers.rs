//! The triggers the light reacts to.

use statechart::trigger::Event;

/// Flips the switch. Both states handle it.
pub struct Toggle;
impl Event for Toggle {
    type Output = ();
}
