use crate::door::Door;
use crate::door_locked::Door_Locked;
use crate::door_open::Door_Open;
use crate::triggers::{Lock, Open};
use statechart::prelude::*;

/// Shut but not locked. Has no hooks of its own.
#[derive(Debug, Default)]
pub struct Door_Closed;

impl State for Door_Closed {
    type Parent = Door;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Open>().trigger::<Lock>();
    }
}

impl Trigger<Open> for Door_Closed {
    fn handle(&mut self, _: &Open, ctx: &mut Context<Self>) {
        ctx.transition(Door_Open);
    }
}

impl Trigger<Lock> for Door_Closed {
    fn handle(&mut self, lock: &Lock, ctx: &mut Context<Self>) {
        // The trigger's data becomes the new state's data.
        ctx.transition(Door_Locked { code: lock.code });
    }
}
