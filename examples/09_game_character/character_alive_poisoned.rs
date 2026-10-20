use crate::character::Character;
use crate::character_alive::Character_Alive;
use crate::character_alive_healthy::Character_Alive_Healthy;
use crate::character_alive_standing::Character_Alive_Standing;
use crate::character_alive_walking::Character_Alive_Walking;
use crate::character_dead::Character_Dead;
use crate::triggers::Describe;
use statechart::prelude::*;

/// How much health the poison takes every update.
pub const POISON_DAMAGE: i32 = 10;

/// Poisoned for `ticks_left` more updates, each of which costs `POISON_DAMAGE` health.
#[derive(Debug)]
pub struct Character_Alive_Poisoned {
    pub ticks_left: u32,
}

impl Default for Character_Alive_Poisoned {
    fn default() -> Self {
        Character_Alive_Poisoned { ticks_left: 3 }
    }
}

impl State for Character_Alive_Poisoned {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Describe>();
    }

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        // A request for another region: the movement region stops walking.
        if ctx.is_active::<Character_Alive_Walking>() {
            ctx.transition(Character_Alive_Standing);
        }
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        let character = ctx.ancestor_mut::<Character>().expect("the root");
        character.health = (character.health - POISON_DAMAGE).max(0);
        let dead = character.health == 0;
        self.ticks_left -= 1;
        if dead {
            ctx.transition(Character_Dead);
        } else if self.ticks_left == 0 {
            ctx.transition(Character_Alive_Healthy);
        }
    }
}

impl Trigger<Describe> for Character_Alive_Poisoned {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        format!("poisoned ({} ticks left)", self.ticks_left)
    }
}
