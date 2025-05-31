use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::Any;

pub trait CustomStateTrait : Any {
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_children(&self) -> Vec<State> {
        vec![]
    }
}
