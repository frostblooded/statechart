use crate::media_player_off::MediaPlayer_Off;
use crate::media_player_on::MediaPlayer_On;
use statechart::prelude::*;

/// The root.
#[derive(Debug, Default)]
pub struct MediaPlayer;

impl State for MediaPlayer {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<MediaPlayer_Off>().child::<MediaPlayer_On>();
    }
}
