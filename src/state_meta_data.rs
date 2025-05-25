use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::TypeId;

pub struct StateMetaData {
    pub children_type_ids: Vec<TypeId>,
    pub children: Vec<State>,
    pub active_children_idx: Vec<usize>
}

impl StateMetaData {
    pub fn new(children: Vec<State>, children_type_ids: Vec<TypeId>) -> Self {
        let active_children_idx: Vec<usize> = if children.is_empty() { vec! [] } else { vec![0] };
        StateMetaData { children, active_children_idx, children_type_ids }
    }

    pub fn update(&mut self, context: &mut StatechartUpdateContext) {
        for child_idx in &self.active_children_idx {
            if let Some(child) = self.children.get_mut(*child_idx) {
                child.update(context);
            }
        }
    }
}
