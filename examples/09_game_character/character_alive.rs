use crate::character::Character;
use crate::character_alive_bow::Character_Alive_Bow;
use crate::character_alive_healthy::Character_Alive_Healthy;
use crate::character_alive_jumping::Character_Alive_Jumping;
use crate::character_alive_poisoned::Character_Alive_Poisoned;
use crate::character_alive_standing::Character_Alive_Standing;
use crate::character_alive_sword::Character_Alive_Sword;
use crate::character_alive_unarmed::Character_Alive_Unarmed;
use crate::character_alive_walking::Character_Alive_Walking;
use crate::character_dead::Character_Dead;
use crate::triggers::{Damage, EquipBow, EquipSword};
use statechart::prelude::*;

/// Alive: moving, holding a weapon and in some health condition, all at once.
#[derive(Debug, Default)]
pub struct Character_Alive;

impl State for Character_Alive {
    type Parent = Character;

    fn define(def: &mut Definition<Self>) {
        // Movement.
        def.region::<Character_Alive_Standing>()
            .child::<Character_Alive_Walking>()
            .child::<Character_Alive_Jumping>();
        // Weapon.
        def.region::<Character_Alive_Unarmed>()
            .child::<Character_Alive_Sword>()
            .child_explicit::<Character_Alive_Bow>();
        // Status.
        def.region::<Character_Alive_Healthy>()
            .child::<Character_Alive_Poisoned>();
        def.trigger::<Damage>()
            .trigger::<EquipSword>()
            .trigger::<EquipBow>();
    }
}

/// How hard an attack hits: twice the weapon's `base` damage in mid-air. The weapons call it
/// with `ctx.is_active::<Character_Alive_Jumping>()`: any state can ask whether a state in
/// another region is active.
pub fn attack_damage(base: u32, jumping: bool) -> u32 {
    if jumping { base * 2 } else { base }
}

impl Trigger<Damage> for Character_Alive {
    fn handle(&mut self, damage: &Damage, ctx: &mut Context<Self>) {
        let character = ctx.parent_mut();
        character.health = (character.health - damage.0).max(0);
        if character.health == 0 {
            // Exits all three regions.
            ctx.transition(Character_Dead);
        }
    }
}

impl Trigger<EquipSword> for Character_Alive {
    // The parent picks the weapon: only the weapon region changes, the others keep going.
    fn handle(&mut self, _: &EquipSword, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Sword);
    }
}

impl Trigger<EquipBow> for Character_Alive {
    fn handle(&mut self, bow: &EquipBow, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Bow { arrows: bow.arrows });
    }
}
