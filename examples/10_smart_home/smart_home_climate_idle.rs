use crate::smart_home::SmartHome;
use crate::smart_home_climate::SmartHome_Climate;
use crate::smart_home_climate_cooling::SmartHome_Climate_Cooling;
use crate::smart_home_climate_heating::SmartHome_Climate_Heating;
use statechart::prelude::*;

/// At the target temperature: compares it with the house every update.
#[derive(Debug, Default)]
pub struct SmartHome_Climate_Idle;

impl State for SmartHome_Climate_Idle {
    type Parent = SmartHome_Climate;

    fn update(&mut self, ctx: &mut Context<Self>) {
        let target = ctx.parent().target;
        let indoor = ctx.ancestor::<SmartHome>().expect("the root").indoor;
        if indoor < target {
            ctx.transition(SmartHome_Climate_Heating);
        } else if indoor > target {
            ctx.transition(SmartHome_Climate_Cooling);
        }
    }
}
