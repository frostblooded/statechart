use crate::statechart_update_context::StatechartUpdateContext;
use std::any::{Any, TypeId};

pub trait CustomStateTrait : Any {
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_possible_children_type_ids(&self) -> Vec<TypeId> {
        vec![]
    }

    fn get_initial_child(&self) -> Option<Box<dyn CustomStateTrait>> {
        None
    }
}
