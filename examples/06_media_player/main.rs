//! `cargo run --example 06_media_player`: trigger fallback. A trigger bubbles up from the active
//! leaf until a state handles it, so `MediaPlayer_On` handles `Stop`, `Next` and `Power` once
//! for all three of its children, and answers `Describe` for the children that do not.
//!
//! ```text
//! MediaPlayer
//! └── region: MediaPlayer_Off (initial), MediaPlayer_On { track }
//!     MediaPlayer_On
//!     └── region: MediaPlayer_On_Stopped (initial), MediaPlayer_On_Playing { position },
//!                 MediaPlayer_On_Paused { position } (explicit)
//! ```
#![allow(non_camel_case_types)]

mod media_player;
mod media_player_off;
mod media_player_on;
mod media_player_on_paused;
mod media_player_on_playing;
mod media_player_on_stopped;
mod triggers;

use media_player::MediaPlayer;
use statechart::prelude::*;
use triggers::{Describe, Next, Pause, Play, Power, Stop};

fn describe(chart: &mut Statechart<MediaPlayer>) {
    println!("  {chart:?}");
    println!("  Describe -> {:?}", chart.trigger(Describe));
}

fn main() {
    let mut chart = Statechart::new(MediaPlayer);
    describe(&mut chart);

    println!("Power -> {:?}", chart.trigger(Power));
    describe(&mut chart);
    println!("Play -> {:?}", chart.trigger(Play));
    chart.update();
    chart.update();
    describe(&mut chart);
    println!("Pause -> {:?}", chart.trigger(Pause));
    describe(&mut chart);
    println!("Next -> {:?}", chart.trigger(Next));
    println!("Play -> {:?}", chart.trigger(Play));
    chart.update();
    describe(&mut chart);
    println!("Stop -> {:?}", chart.trigger(Stop));
    describe(&mut chart);
    println!("Power -> {:?}", chart.trigger(Power));
    describe(&mut chart);
}
