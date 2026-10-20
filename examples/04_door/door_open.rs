use crate::door::Door;
use crate::door_closed::Door_Closed;
use crate::triggers::Close;
use statechart::prelude::*;

/// Wide open.
#[derive(Debug, Default)]
pub struct Door_Open;

impl State for Door_Open {
    type Parent = Door;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Close>();
    }

    // Runs each time the state is entered, before anything else runs on it.
    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        ctx.parent_mut().log.push("door swings open".to_string());
    }

    // Runs when the state is left; the state is dropped right after.
    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        ctx.parent_mut().log.push("door swings shut".to_string());
    }
}

impl Trigger<Close> for Door_Open {
    fn handle(&mut self, _: &Close, ctx: &mut Context<Self>) {
        ctx.transition(Door_Closed);
    }
}
