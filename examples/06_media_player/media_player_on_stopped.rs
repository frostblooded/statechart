use crate::media_player_on::MediaPlayer_On;
use crate::media_player_on_playing::MediaPlayer_On_Playing;
use crate::triggers::Play;
use statechart::prelude::*;

/// Not playing. Handles only `Play`; everything else falls back to `MediaPlayer_On`.
#[derive(Debug, Default)]
pub struct MediaPlayer_On_Stopped;

impl State for MediaPlayer_On_Stopped {
    type Parent = MediaPlayer_On;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Play>();
    }
}

impl Trigger<Play> for MediaPlayer_On_Stopped {
    fn handle(&mut self, _: &Play, ctx: &mut Context<Self>) {
        ctx.transition(MediaPlayer_On_Playing::default());
    }
}
