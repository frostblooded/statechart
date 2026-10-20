use crate::character_alive::{Character_Alive, attack_damage};
use crate::character_alive_jumping::Character_Alive_Jumping;
use crate::character_alive_unarmed::Character_Alive_Unarmed;
use crate::triggers::{Attack, Describe};
use statechart::prelude::*;

/// Holding a bow with `arrows` left. Only entered with `EquipBow { arrows }`, which says how
/// many.
#[derive(Debug)]
pub struct Character_Alive_Bow {
    pub arrows: u32,
}

impl State for Character_Alive_Bow {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Attack>().trigger::<Describe>();
    }
}

impl Trigger<Attack> for Character_Alive_Bow {
    fn handle(&mut self, _: &Attack, ctx: &mut Context<Self>) -> u32 {
        self.arrows -= 1;
        if self.arrows == 0 {
            // Out of arrows: drop the bow.
            ctx.transition(Character_Alive_Unarmed);
        }
        attack_damage(15, ctx.is_active::<Character_Alive_Jumping>())
    }
}

impl Trigger<Describe> for Character_Alive_Bow {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        format!("bow ({} arrows)", self.arrows)
    }
}
