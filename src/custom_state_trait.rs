use crate::state_trait::StateTrait;
use crate::statechart_update_context::StatechartUpdateContext;

pub trait CustomStateTrait {
    fn new() -> Self where Self: Sized;
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn get_children() -> Vec<Box<dyn StateTrait>> where Self: Sized {
        vec![]
    }
}
