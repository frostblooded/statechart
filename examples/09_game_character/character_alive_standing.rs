use crate::character_alive::Character_Alive;
use crate::character_alive_jumping::Character_Alive_Jumping;
use crate::character_alive_poisoned::Character_Alive_Poisoned;
use crate::character_alive_walking::Character_Alive_Walking;
use crate::triggers::{Describe, Jump, Walk};
use statechart::prelude::*;

/// Standing still.
#[derive(Debug, Default)]
pub struct Character_Alive_Standing;

impl State for Character_Alive_Standing {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Walk>()
            .trigger::<Jump>()
            .trigger::<Describe>();
    }
}

impl Trigger<Walk> for Character_Alive_Standing {
    /// Whether the character started walking.
    fn handle(&mut self, _: &Walk, ctx: &mut Context<Self>) -> bool {
        // The status region decides: poison makes walking impossible.
        if ctx.is_active::<Character_Alive_Poisoned>() {
            return false;
        }
        ctx.transition(Character_Alive_Walking);
        true
    }
}

impl Trigger<Jump> for Character_Alive_Standing {
    fn handle(&mut self, _: &Jump, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Jumping::default());
    }
}

impl Trigger<Describe> for Character_Alive_Standing {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "standing".to_string()
    }
}
