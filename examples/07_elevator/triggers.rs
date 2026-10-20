//! The triggers the elevator reacts to.

use statechart::trigger::Event;

/// Calls the car to a floor. Answers whether the floor exists.
pub struct Call(pub i32);
impl Event for Call {
    type Output = bool;
}

/// Takes the elevator out of service, from any state.
pub struct StartMaintenance;
impl Event for StartMaintenance {
    type Output = ();
}

/// Puts the elevator back in service; it first returns to the ground floor.
pub struct Resume;
impl Event for Resume {
    type Output = ();
}
