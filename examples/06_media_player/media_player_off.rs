use crate::media_player::MediaPlayer;
use crate::media_player_on::MediaPlayer_On;
use crate::triggers::Power;
use statechart::prelude::*;

/// Switched off. Only `Power` does anything.
#[derive(Debug, Default)]
pub struct MediaPlayer_Off;

impl State for MediaPlayer_Off {
    type Parent = MediaPlayer;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Power>();
    }
}

impl Trigger<Power> for MediaPlayer_Off {
    fn handle(&mut self, _: &Power, ctx: &mut Context<Self>) {
        // Entering On also enters its initial child, Stopped.
        ctx.transition(MediaPlayer_On::default());
    }
}
