// Runs the state code from the `09_game_character` example: three parallel regions, one
// trigger answered by every region (`trigger_all`), `is_active` checks across regions and a
// transition requested in one region for another.
#![allow(non_camel_case_types)]

#[path = "../examples/09_game_character/character.rs"]
mod character;
#[path = "../examples/09_game_character/character_alive.rs"]
mod character_alive;
#[path = "../examples/09_game_character/character_alive_bow.rs"]
mod character_alive_bow;
#[path = "../examples/09_game_character/character_alive_healthy.rs"]
mod character_alive_healthy;
#[path = "../examples/09_game_character/character_alive_jumping.rs"]
mod character_alive_jumping;
#[path = "../examples/09_game_character/character_alive_poisoned.rs"]
mod character_alive_poisoned;
#[path = "../examples/09_game_character/character_alive_standing.rs"]
mod character_alive_standing;
#[path = "../examples/09_game_character/character_alive_sword.rs"]
mod character_alive_sword;
#[path = "../examples/09_game_character/character_alive_unarmed.rs"]
mod character_alive_unarmed;
#[path = "../examples/09_game_character/character_alive_walking.rs"]
mod character_alive_walking;
#[path = "../examples/09_game_character/character_dead.rs"]
mod character_dead;
#[path = "../examples/09_game_character/triggers.rs"]
mod triggers;

use character::Character;
use character_alive::Character_Alive;
use character_alive_bow::Character_Alive_Bow;
use character_alive_healthy::Character_Alive_Healthy;
use character_alive_jumping::Character_Alive_Jumping;
use character_alive_poisoned::Character_Alive_Poisoned;
use character_alive_standing::Character_Alive_Standing;
use character_alive_sword::Character_Alive_Sword;
use character_alive_unarmed::Character_Alive_Unarmed;
use character_alive_walking::Character_Alive_Walking;
use character_dead::Character_Dead;
use statechart::prelude::*;
use triggers::{Attack, Damage, Describe, EquipBow, EquipSword, Jump, Poison, Revive, Stop, Walk};

fn describe_all(chart: &mut Statechart<Character>) -> Vec<String> {
    chart.trigger_all(Describe).unwrap()
}

#[test]
fn test_starts_alive_with_three_regions() {
    let mut chart = Statechart::new(Character::new());
    assert_eq!(
        chart.get_debug_name(),
        "Character { health: 100 } -> Character_Alive -> [Character_Alive_Standing, \
         Character_Alive_Unarmed, Character_Alive_Healthy]"
    );
    // Every region answers, in declaration order; `trigger` keeps only the first answer.
    assert_eq!(describe_all(&mut chart), ["standing", "unarmed", "healthy"]);
    assert_eq!(chart.trigger(Describe), Ok("standing".to_string()));
}

#[test]
fn test_movement_walk_jump_land() {
    let mut chart = Statechart::new(Character::new());
    assert_eq!(chart.trigger(Walk), Ok(true));
    assert!(chart.is_active::<Character_Alive_Walking>());
    chart.trigger(Jump).unwrap();
    assert!(chart.is_active::<Character_Alive_Jumping>());
    for _ in 0..character_alive_jumping::AIRTIME {
        chart.update();
    }
    assert!(chart.is_active::<Character_Alive_Standing>());
    chart.trigger(Walk).unwrap();
    chart.trigger(Stop).unwrap();
    assert!(chart.is_active::<Character_Alive_Standing>());
    assert_eq!(chart.trigger(Stop), Err(NotHandled));
}

#[test]
fn test_weapons_change_only_their_region() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(Walk).unwrap();
    assert_eq!(chart.trigger(Attack), Ok(1));
    chart.trigger(EquipSword).unwrap();
    assert_eq!(chart.trigger(Attack), Ok(10));
    assert_eq!(describe_all(&mut chart), ["walking", "sword", "healthy"]);
}

#[test]
fn test_attacks_while_jumping_hit_twice_as_hard() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(EquipSword).unwrap();
    chart.trigger(Jump).unwrap();
    assert_eq!(chart.trigger(Attack), Ok(20));
}

#[test]
fn test_the_bow_runs_out_of_arrows() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(EquipBow { arrows: 2 }).unwrap();
    assert_eq!(chart.state::<Character_Alive_Bow>().unwrap().arrows, 2);
    assert_eq!(chart.trigger(Attack), Ok(15));
    assert_eq!(describe_all(&mut chart)[1], "bow (1 arrows)");
    assert_eq!(chart.trigger(Attack), Ok(15));
    // The last arrow is gone: back to fists.
    assert!(chart.is_active::<Character_Alive_Unarmed>());
}

#[test]
fn test_poison_stops_walking_and_prevents_it() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(Walk).unwrap();
    chart.trigger(Poison).unwrap();
    // Poisoned's on_enter stopped the walk, in the movement region.
    assert!(chart.is_active::<Character_Alive_Poisoned>());
    assert!(chart.is_active::<Character_Alive_Standing>());
    assert_eq!(chart.trigger(Walk), Ok(false));
    assert!(chart.is_active::<Character_Alive_Standing>());
}

#[test]
fn test_poison_hurts_every_update_then_wears_off() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(Poison).unwrap();
    assert_eq!(chart.trigger(Poison), Err(NotHandled));
    chart.update();
    assert_eq!(chart.root().health, 90);
    assert_eq!(
        describe_all(&mut chart),
        ["standing", "unarmed", "poisoned (2 ticks left)"]
    );
    chart.update();
    chart.update();
    assert_eq!(chart.root().health, 70);
    assert!(chart.is_active::<Character_Alive_Healthy>());
    assert_eq!(chart.trigger(Walk), Ok(true));
}

#[test]
fn test_damage_kills_and_revive_starts_fresh() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(EquipSword).unwrap();
    chart.trigger(Damage(40)).unwrap();
    assert_eq!(chart.root().health, 60);
    assert!(chart.is_active::<Character_Alive>());
    chart.trigger(Damage(60)).unwrap();
    assert_eq!(
        chart.get_debug_name(),
        "Character { health: 0 } -> Character_Dead"
    );
    assert_eq!(describe_all(&mut chart), ["dead"]);
    assert_eq!(chart.trigger(Attack), Err(NotHandled));
    assert_eq!(chart.trigger(Damage(1)), Err(NotHandled));

    chart.trigger(Revive).unwrap();
    assert_eq!(chart.root().health, character::MAX_HEALTH);
    assert!(chart.is_active::<Character_Alive_Unarmed>());
    assert!(!chart.is_active::<Character_Alive_Sword>());
    assert!(!chart.is_active::<Character_Dead>());
}

#[test]
fn test_poison_can_kill() {
    let mut chart = Statechart::new(Character::new());
    chart.trigger(Damage(85)).unwrap();
    chart.trigger(Poison).unwrap();
    chart.update();
    assert!(chart.is_active::<Character_Alive_Poisoned>());
    chart.update();
    assert!(chart.is_active::<Character_Dead>());
}
