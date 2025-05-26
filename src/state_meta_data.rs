use crate::state::State;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::TypeId;

pub struct StateMetaData {
    children_type_ids: Vec<TypeId>,
    children: Vec<State>,
    active_children_idx: Option<usize>,
}

impl StateMetaData {
    pub fn new(children: Vec<State>, children_type_ids: Vec<TypeId>) -> Self {
        let active_children_idx: Option<usize> = if children.is_empty() { None } else { Some(0) };
        StateMetaData { children, active_children_idx, children_type_ids }
    }

    pub fn update(&mut self, context: &mut StatechartUpdateContext) {
        self.update_children(context);
        self.apply_transitions(context);
    }

    fn update_children(&mut self, context: &mut StatechartUpdateContext) {
        if let Some(child_idx) = self.active_children_idx {
            if let Some(child) = self.children.get_mut(child_idx) {
                child.update(context);
            }
        }
    }

    fn apply_transitions(&mut self, context: &StatechartUpdateContext) {
        for transition_id in &context.transitions {
            for idx in 0..self.children.len() {
                let type_id: TypeId = self.children_type_ids[idx];

                if type_id == *transition_id {
                    self.active_children_idx = Some(idx);
                }
            }
        }
    }
}
