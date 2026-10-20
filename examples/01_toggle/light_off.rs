use crate::light::Light;
use crate::light_on::Light_On;
use crate::triggers::Toggle;
use statechart::prelude::*;

/// The light is off.
#[derive(Debug, Default)]
pub struct Light_Off;

impl State for Light_Off {
    type Parent = Light;

    fn define(def: &mut Definition<Self>) {
        // Registering the trigger is what makes `handle` below run.
        def.trigger::<Toggle>();
    }
}

impl Trigger<Toggle> for Light_Off {
    fn handle(&mut self, _: &Toggle, ctx: &mut Context<Self>) {
        // Applied once the trigger has been handled.
        ctx.transition(Light_On);
    }
}
