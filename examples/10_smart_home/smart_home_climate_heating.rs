use crate::smart_home::SmartHome;
use crate::smart_home_climate::SmartHome_Climate;
use crate::smart_home_climate_idle::SmartHome_Climate_Idle;
use statechart::prelude::*;

/// Heating: the house gets 1 °C warmer every update until it reaches the target.
#[derive(Debug, Default)]
pub struct SmartHome_Climate_Heating;

impl State for SmartHome_Climate_Heating {
    type Parent = SmartHome_Climate;

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        ctx.ancestor_mut::<SmartHome>()
            .expect("the root")
            .note("heating on");
    }

    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        ctx.ancestor_mut::<SmartHome>()
            .expect("the root")
            .note("heating off");
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        let target = ctx.parent().target;
        let home = ctx.ancestor_mut::<SmartHome>().expect("the root");
        home.indoor += 1;
        if home.indoor >= target {
            ctx.transition(SmartHome_Climate_Idle);
        }
    }
}
