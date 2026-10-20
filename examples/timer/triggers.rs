//! The triggers the timer reacts to.

use statechart::trigger::Event;
use std::time::Duration;

/// Pauses a running countdown. Only `Timer_Active_Running` handles it.
pub struct Pause;
impl Event for Pause {
    type Output = ();
}

/// Resumes a paused countdown. Only `Timer_Active_Paused` handles it.
pub struct Resume;
impl Event for Resume {
    type Output = ();
}

/// Starts the countdown over from the full duration, whatever the timer is doing. Handled by
/// the root, `Timer`.
pub struct Reset;
impl Event for Reset {
    type Output = ();
}

/// How much time is left. `Timer_Active_Running` answers with the live value;
/// `Timer_Active_Paused` falls back to `Timer_Active`; once elapsed, nobody handles it.
pub struct GetRemaining;
impl Event for GetRemaining {
    type Output = Duration;
}
