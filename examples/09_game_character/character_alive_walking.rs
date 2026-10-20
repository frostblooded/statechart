use crate::character_alive::Character_Alive;
use crate::character_alive_jumping::Character_Alive_Jumping;
use crate::character_alive_standing::Character_Alive_Standing;
use crate::triggers::{Describe, Jump, Stop};
use statechart::prelude::*;

/// Walking.
#[derive(Debug, Default)]
pub struct Character_Alive_Walking;

impl State for Character_Alive_Walking {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Stop>()
            .trigger::<Jump>()
            .trigger::<Describe>();
    }
}

impl Trigger<Stop> for Character_Alive_Walking {
    fn handle(&mut self, _: &Stop, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Standing);
    }
}

impl Trigger<Jump> for Character_Alive_Walking {
    fn handle(&mut self, _: &Jump, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Jumping::default());
    }
}

impl Trigger<Describe> for Character_Alive_Walking {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "walking".to_string()
    }
}
