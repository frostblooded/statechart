use crate::character_alive::Character_Alive;
use crate::character_dead::Character_Dead;
use statechart::prelude::*;

/// Health when the character is created or revived.
pub const MAX_HEALTH: i32 = 100;

/// The root: the character's `health`.
#[derive(Debug)]
pub struct Character {
    pub health: i32,
}

impl Character {
    pub fn new() -> Self {
        Character { health: MAX_HEALTH }
    }
}

impl Default for Character {
    fn default() -> Self {
        Character::new()
    }
}

impl State for Character {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<Character_Alive>().child::<Character_Dead>();
    }
}
