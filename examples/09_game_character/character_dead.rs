use crate::character::{Character, MAX_HEALTH};
use crate::character_alive::Character_Alive;
use crate::triggers::{Describe, Revive};
use statechart::prelude::*;

/// Dead: none of `Character_Alive`'s regions exist, so nothing but `Describe` and `Revive` is
/// handled.
#[derive(Debug, Default)]
pub struct Character_Dead;

impl State for Character_Dead {
    type Parent = Character;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Revive>().trigger::<Describe>();
    }
}

impl Trigger<Revive> for Character_Dead {
    fn handle(&mut self, _: &Revive, ctx: &mut Context<Self>) {
        ctx.parent_mut().health = MAX_HEALTH;
        // Each region starts over from its initial child: standing, unarmed, healthy.
        ctx.transition(Character_Alive);
    }
}

impl Trigger<Describe> for Character_Dead {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "dead".to_string()
    }
}
