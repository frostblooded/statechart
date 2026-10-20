use crate::character_alive::Character_Alive;
use crate::character_alive_poisoned::Character_Alive_Poisoned;
use crate::triggers::{Describe, Poison};
use statechart::prelude::*;

/// Nothing wrong.
#[derive(Debug, Default)]
pub struct Character_Alive_Healthy;

impl State for Character_Alive_Healthy {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Poison>().trigger::<Describe>();
    }
}

impl Trigger<Poison> for Character_Alive_Healthy {
    fn handle(&mut self, _: &Poison, ctx: &mut Context<Self>) {
        ctx.transition(Character_Alive_Poisoned::default());
    }
}

impl Trigger<Describe> for Character_Alive_Healthy {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "healthy".to_string()
    }
}
