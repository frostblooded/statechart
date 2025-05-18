use std::any::TypeId;
use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;

pub trait CustomStateTrait {
    fn new() -> Self where Self: Sized;
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_children_and_type_ids() -> (Vec<State>, Vec<TypeId>) where Self: Sized {
        (vec![], vec![])
    }
}
