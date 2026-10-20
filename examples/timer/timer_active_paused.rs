use crate::timer_active::Timer_Active;
use crate::timer_active_running::Timer_Active_Running;
use crate::triggers::Resume;
use statechart::prelude::*;

/// The clock is stopped; `Timer_Active::remaining` does not change.
#[derive(Debug, Default)]
pub struct Timer_Active_Paused;

impl State for Timer_Active_Paused {
    type Parent = Timer_Active;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Resume>();
    }
}

impl Trigger<Resume> for Timer_Active_Paused {
    fn handle(&mut self, _: &Resume, ctx: &mut Context<Self>) {
        ctx.transition(Timer_Active_Running::default());
    }
}
