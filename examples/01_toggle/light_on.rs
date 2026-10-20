use crate::light::Light;
use crate::light_off::Light_Off;
use crate::triggers::Toggle;
use statechart::prelude::*;

/// The light is on.
#[derive(Debug, Default)]
pub struct Light_On;

impl State for Light_On {
    type Parent = Light;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Toggle>();
    }
}

impl Trigger<Toggle> for Light_On {
    fn handle(&mut self, _: &Toggle, ctx: &mut Context<Self>) {
        ctx.transition(Light_Off);
    }
}
