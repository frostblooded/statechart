use crate::door::Door;
use crate::door_closed::Door_Closed;
use crate::triggers::Unlock;
use statechart::prelude::*;

/// Locked with `code`; only `Unlock` with the same code opens the bolt.
#[derive(Debug)]
pub struct Door_Locked {
    pub code: u32,
}

impl State for Door_Locked {
    type Parent = Door;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Unlock>();
    }

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        ctx.parent_mut().log.push("bolt slides in".to_string());
    }

    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        ctx.parent_mut().log.push("bolt slides out".to_string());
    }
}

impl Trigger<Unlock> for Door_Locked {
    /// Whether the code was right.
    fn handle(&mut self, unlock: &Unlock, ctx: &mut Context<Self>) -> bool {
        let right = unlock.code == self.code;
        if right {
            ctx.transition(Door_Closed);
        }
        right
    }
}
