use std::any::{Any, TypeId};
use crate::state_trait::StateTrait;
use crate::statechart_update_context::StatechartUpdateContext;

pub struct StateMetaData {
    pub children_type_ids: Vec<TypeId>,
    pub children: Vec<Box<dyn StateTrait>>,
    pub active_children_idx: Vec<usize>
}

impl StateMetaData {
    pub fn new(children: Vec<Box<dyn StateTrait>>) -> Self {
        let children_type_ids: Vec<TypeId> = children.iter().map(|child: &Box<dyn StateTrait>| child.type_id()).collect();
        let active_children_idx: Vec<usize> = if children.is_empty() { vec! [] } else { vec![0] };
        StateMetaData { children, active_children_idx, children_type_ids }
    }

    pub fn update(&self, context: &mut StatechartUpdateContext) {
        for child_idx in &self.active_children_idx {
            if let Some(child) = &self.children.get(*child_idx) {
                child.update(context);
            }
        }
    }
}
