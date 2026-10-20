use crate::media_player::MediaPlayer;
use crate::media_player_off::MediaPlayer_Off;
use crate::media_player_on_paused::MediaPlayer_On_Paused;
use crate::media_player_on_playing::MediaPlayer_On_Playing;
use crate::media_player_on_stopped::MediaPlayer_On_Stopped;
use crate::triggers::{Describe, Next, Power, Stop};
use statechart::prelude::*;

/// Switched on, on `track`. The fallback for its children: whichever of them is active, a
/// trigger it does not handle bubbles up to here.
#[derive(Debug)]
pub struct MediaPlayer_On {
    pub track: u32,
}

impl Default for MediaPlayer_On {
    fn default() -> Self {
        MediaPlayer_On { track: 1 }
    }
}

impl State for MediaPlayer_On {
    type Parent = MediaPlayer;

    fn define(def: &mut Definition<Self>) {
        def.region::<MediaPlayer_On_Stopped>()
            .child::<MediaPlayer_On_Playing>()
            .child_explicit::<MediaPlayer_On_Paused>();
        def.trigger::<Power>()
            .trigger::<Stop>()
            .trigger::<Next>()
            .trigger::<Describe>();
    }
}

impl Trigger<Power> for MediaPlayer_On {
    fn handle(&mut self, _: &Power, ctx: &mut Context<Self>) {
        ctx.transition(MediaPlayer_Off);
    }
}

impl Trigger<Stop> for MediaPlayer_On {
    fn handle(&mut self, _: &Stop, ctx: &mut Context<Self>) {
        ctx.transition(MediaPlayer_On_Stopped);
    }
}

impl Trigger<Next> for MediaPlayer_On {
    // No transition: whatever child is active stays active.
    fn handle(&mut self, _: &Next, _: &mut Context<Self>) -> u32 {
        self.track += 1;
        self.track
    }
}

impl Trigger<Describe> for MediaPlayer_On {
    // Playing answers Describe itself, so this only runs for Stopped and Paused.
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        format!("track {}", self.track)
    }
}
