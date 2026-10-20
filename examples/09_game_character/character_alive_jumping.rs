use crate::character_alive::Character_Alive;
use crate::character_alive_standing::Character_Alive_Standing;
use crate::triggers::Describe;
use statechart::prelude::*;

/// How many updates a jump lasts.
pub const AIRTIME: u32 = 2;

/// In the air; lands (standing) after `AIRTIME` updates.
#[derive(Debug, Default)]
pub struct Character_Alive_Jumping {
    pub ticks: u32,
}

impl State for Character_Alive_Jumping {
    type Parent = Character_Alive;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Describe>();
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        self.ticks += 1;
        if self.ticks == AIRTIME {
            ctx.transition(Character_Alive_Standing);
        }
    }
}

impl Trigger<Describe> for Character_Alive_Jumping {
    fn handle(&mut self, _: &Describe, _: &mut Context<Self>) -> String {
        "jumping".to_string()
    }
}
