use std::any::TypeId;
use crate::statechart_update_context::StatechartUpdateContext;

pub trait StateBehavior {
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn transition<NewStateType: 'static>(&self, context: &mut StatechartUpdateContext) where Self: Sized {
        let new_state_type_id = TypeId::of::<NewStateType>();
        context.transitions.push(new_state_type_id);
    }
}
