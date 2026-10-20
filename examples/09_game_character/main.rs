//! `cargo run --example 09_game_character`: three parallel regions that work together. One
//! trigger can be answered by every region (`trigger_all`), a state checks what another
//! region is doing (`ctx.is_active`), and a state in one region requests a transition in
//! another.
//!
//! ```text
//! Character { health }
//! └── region: Character_Alive (initial), Character_Dead
//!     Character_Alive
//!     ├── movement: Character_Alive_Standing (initial), Character_Alive_Walking,
//!     │             Character_Alive_Jumping { ticks }
//!     ├── weapon:   Character_Alive_Unarmed (initial), Character_Alive_Sword,
//!     │             Character_Alive_Bow { arrows } (explicit)
//!     └── status:   Character_Alive_Healthy (initial), Character_Alive_Poisoned { ticks_left }
//! ```
#![allow(non_camel_case_types)]

mod character;
mod character_alive;
mod character_alive_bow;
mod character_alive_healthy;
mod character_alive_jumping;
mod character_alive_poisoned;
mod character_alive_standing;
mod character_alive_sword;
mod character_alive_unarmed;
mod character_alive_walking;
mod character_dead;
mod triggers;

use character::Character;
use statechart::prelude::*;
use triggers::{Attack, Damage, Describe, EquipBow, EquipSword, Jump, Poison, Revive, Walk};

fn status(chart: &mut Statechart<Character>) {
    // One answer per region that handles Describe.
    let parts = chart
        .trigger_all(Describe)
        .expect("every state handles Describe");
    println!("  health {}: {}", chart.root().health, parts.join(", "));
}

fn main() {
    let mut chart = Statechart::new(Character::new());
    println!("{chart:?}");
    status(&mut chart);

    println!("Walk -> {:?}", chart.trigger(Walk));
    println!("EquipSword -> {:?}", chart.trigger(EquipSword));
    println!("Attack -> {:?}", chart.trigger(Attack));
    println!("Jump -> {:?}", chart.trigger(Jump));
    // Sword and jump together: the sword checks the movement region.
    println!("Attack -> {:?}", chart.trigger(Attack));
    status(&mut chart);
    chart.update();
    chart.update();
    status(&mut chart);

    println!("Walk -> {:?}", chart.trigger(Walk));
    // Poisoned's on_enter stops the walk in the movement region.
    println!("Poison -> {:?}", chart.trigger(Poison));
    status(&mut chart);
    println!("Walk -> {:?}", chart.trigger(Walk));
    for _ in 0..3 {
        chart.update();
        status(&mut chart);
    }

    println!("EquipBow -> {:?}", chart.trigger(EquipBow { arrows: 1 }));
    println!("Attack -> {:?}", chart.trigger(Attack));
    status(&mut chart);

    println!("Damage(80) -> {:?}", chart.trigger(Damage(80)));
    println!("  {chart:?}");
    status(&mut chart);
    println!("Attack -> {:?}", chart.trigger(Attack));
    println!("Revive -> {:?}", chart.trigger(Revive));
    status(&mut chart);
}
