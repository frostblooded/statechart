use crate::media_player_on::MediaPlayer_On;
use crate::media_player_on_paused::MediaPlayer_On_Paused;
use crate::triggers::{Describe, Pause};
use statechart::prelude::*;

/// Playing; `position` (in seconds) moves on with every update.
#[derive(Debug, Default)]
pub struct MediaPlayer_On_Playing {
    pub position: u32,
}

impl State for MediaPlayer_On_Playing {
    type Parent = MediaPlayer_On;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Pause>().trigger::<Describe>();
    }

    fn update(&mut self, _: &mut Context<Self>) {
        self.position += 1;
    }
}

impl Trigger<Pause> for MediaPlayer_On_Playing {
    fn handle(&mut self, _: &Pause, ctx: &mut Context<Self>) {
        ctx.transition(MediaPlayer_On_Paused {
            position: self.position,
        });
    }
}

impl Trigger<Describe> for MediaPlayer_On_Playing {
    // The deepest handler wins: MediaPlayer_On's Describe does not run while playing.
    fn handle(&mut self, _: &Describe, ctx: &mut Context<Self>) -> String {
        format!("playing track {} at {}s", ctx.parent().track, self.position)
    }
}
