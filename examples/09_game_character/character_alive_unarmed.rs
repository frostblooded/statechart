use crate::character_alive::{Character_Alive, attack_damage};
use crate::character_alive_jumping::Character_Alive_Jumping;
use crate::triggers::{Attack, Describe};
use statechart::prelude::*;

/// No weapon: fists.
#[derive(Debug, Default)]
pub struct Character_Alive_Unarmed;

impl State for Character_Alive_Unarmed {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Attack>().trigger::<Describe>();
    }
}

impl Trigger<Attack> for Character_Alive_Unarmed {
    fn handle(&mut self, _: &Attack, ctx: &mut Context<Self>) -> u32 {
        attack_damage(1, ctx.is_active::<Character_Alive_Jumping>())
    }
}

impl Trigger<Describe> for Character_Alive_Unarmed {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "unarmed".to_string()
    }
}
