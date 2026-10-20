//! The triggers the door reacts to.

use statechart::trigger::Event;

/// Opens a closed door.
pub struct Open;
impl Event for Open {
    type Output = ();
}

/// Closes an open door.
pub struct Close;
impl Event for Close {
    type Output = ();
}

/// Locks a closed door with a code.
pub struct Lock {
    pub code: u32,
}
impl Event for Lock {
    type Output = ();
}

/// Tries to unlock a locked door. Answers whether the code was right.
pub struct Unlock {
    pub code: u32,
}
impl Event for Unlock {
    type Output = bool;
}
