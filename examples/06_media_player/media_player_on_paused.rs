use crate::media_player_on::MediaPlayer_On;
use crate::media_player_on_playing::MediaPlayer_On_Playing;
use crate::triggers::Play;
use statechart::prelude::*;

/// Paused at `position`, which `Play` resumes from.
#[derive(Debug)]
pub struct MediaPlayer_On_Paused {
    pub position: u32,
}

impl State for MediaPlayer_On_Paused {
    type Parent = MediaPlayer_On;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Play>();
    }
}

impl Trigger<Play> for MediaPlayer_On_Paused {
    fn handle(&mut self, _: &Play, ctx: &mut Context<Self>) {
        ctx.transition(MediaPlayer_On_Playing {
            position: self.position,
        });
    }
}
