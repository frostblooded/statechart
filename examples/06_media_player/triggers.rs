//! The triggers the media player reacts to.

use statechart::trigger::Event;

/// Switches the player on or off.
pub struct Power;
impl Event for Power {
    type Output = ();
}

/// Starts or resumes playback.
pub struct Play;
impl Event for Play {
    type Output = ();
}

/// Pauses playback.
pub struct Pause;
impl Event for Pause {
    type Output = ();
}

/// Stops playback. Handled by `MediaPlayer_On` for all of its children.
pub struct Stop;
impl Event for Stop {
    type Output = ();
}

/// Skips to the next track. Answers the new track number.
pub struct Next;
impl Event for Next {
    type Output = u32;
}

/// Describes what the player is doing.
pub struct Describe;
impl Event for Describe {
    type Output = String;
}
