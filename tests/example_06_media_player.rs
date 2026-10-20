// Runs the state code from the `06_media_player` example: triggers that fall back to an
// ancestor when the active child does not handle them.
#![allow(non_camel_case_types)]

#[path = "../examples/06_media_player/media_player.rs"]
mod media_player;
#[path = "../examples/06_media_player/media_player_off.rs"]
mod media_player_off;
#[path = "../examples/06_media_player/media_player_on.rs"]
mod media_player_on;
#[path = "../examples/06_media_player/media_player_on_paused.rs"]
mod media_player_on_paused;
#[path = "../examples/06_media_player/media_player_on_playing.rs"]
mod media_player_on_playing;
#[path = "../examples/06_media_player/media_player_on_stopped.rs"]
mod media_player_on_stopped;
#[path = "../examples/06_media_player/triggers.rs"]
mod triggers;

use media_player::MediaPlayer;
use media_player_off::MediaPlayer_Off;
use media_player_on::MediaPlayer_On;
use media_player_on_paused::MediaPlayer_On_Paused;
use media_player_on_playing::MediaPlayer_On_Playing;
use media_player_on_stopped::MediaPlayer_On_Stopped;
use statechart::prelude::*;
use triggers::{Describe, Next, Pause, Play, Power, Stop};

fn powered_on() -> Statechart<MediaPlayer> {
    let mut chart = Statechart::new(MediaPlayer);
    chart.trigger(Power).unwrap();
    chart
}

#[test]
fn test_starts_off_and_power_turns_it_on_stopped_on_track_one() {
    let mut chart = Statechart::new(MediaPlayer);
    assert_eq!(chart.get_debug_name(), "MediaPlayer -> MediaPlayer_Off");
    // Nothing is on to describe or skip.
    assert_eq!(chart.trigger(Describe), Err(NotHandled));
    assert_eq!(chart.trigger(Next), Err(NotHandled));

    chart.trigger(Power).unwrap();
    assert_eq!(
        chart.get_debug_name(),
        "MediaPlayer -> MediaPlayer_On { track: 1 } -> MediaPlayer_On_Stopped"
    );
}

#[test]
fn test_stopped_falls_back_to_on_for_describe() {
    let mut chart = powered_on();
    assert_eq!(chart.trigger(Describe), Ok("track 1".to_string()));
}

#[test]
fn test_playing_answers_describe_itself() {
    let mut chart = powered_on();
    chart.trigger(Play).unwrap();
    assert_eq!(
        chart.trigger(Describe),
        Ok("playing track 1 at 0s".to_string())
    );
    chart.update();
    chart.update();
    assert_eq!(chart.state::<MediaPlayer_On_Playing>().unwrap().position, 2);
    assert_eq!(
        chart.trigger(Describe),
        Ok("playing track 1 at 2s".to_string())
    );
}

#[test]
fn test_pause_keeps_the_position_and_play_resumes_from_it() {
    let mut chart = powered_on();
    chart.trigger(Play).unwrap();
    chart.update();
    chart.trigger(Pause).unwrap();
    assert_eq!(chart.state::<MediaPlayer_On_Paused>().unwrap().position, 1);
    // Paused updates do not move the position.
    chart.update();
    // Paused does not handle Describe: MediaPlayer_On answers.
    assert_eq!(chart.trigger(Describe), Ok("track 1".to_string()));
    chart.trigger(Play).unwrap();
    assert_eq!(chart.state::<MediaPlayer_On_Playing>().unwrap().position, 1);
}

#[test]
fn test_next_and_stop_are_handled_by_the_parent_from_any_child() {
    let mut chart = powered_on();
    assert_eq!(chart.trigger(Next), Ok(2));
    chart.trigger(Play).unwrap();
    assert_eq!(chart.trigger(Next), Ok(3));
    // The parent handled Next without a transition: still playing.
    assert!(chart.is_active::<MediaPlayer_On_Playing>());
    chart.trigger(Pause).unwrap();
    chart.trigger(Stop).unwrap();
    assert!(chart.is_active::<MediaPlayer_On_Stopped>());
    assert_eq!(chart.state::<MediaPlayer_On>().unwrap().track, 3);
}

#[test]
fn test_power_from_anywhere_turns_it_off_and_back_on_starts_over() {
    let mut chart = powered_on();
    chart.trigger(Next).unwrap();
    chart.trigger(Play).unwrap();
    chart.trigger(Power).unwrap();
    assert!(chart.is_active::<MediaPlayer_Off>());
    chart.trigger(Power).unwrap();
    assert_eq!(chart.state::<MediaPlayer_On>().unwrap().track, 1);
    assert!(chart.is_active::<MediaPlayer_On_Stopped>());
}

#[test]
fn test_play_and_pause_only_where_they_make_sense() {
    let mut chart = powered_on();
    assert_eq!(chart.trigger(Pause), Err(NotHandled));
    chart.trigger(Play).unwrap();
    assert_eq!(chart.trigger(Play), Err(NotHandled));
}
