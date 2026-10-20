//! The triggers the washing machine reacts to.

use statechart::trigger::Event;

/// Starts a cycle.
pub struct Start;
impl Event for Start {
    type Output = ();
}

/// Stops a running cycle, whatever phase it is in.
pub struct Cancel;
impl Event for Cancel {
    type Output = ();
}

/// Takes the clothes out after a cycle.
pub struct Unload;
impl Event for Unload {
    type Output = ();
}
