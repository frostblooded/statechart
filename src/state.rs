use crate::custom_state_trait::CustomStateTrait;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::TypeId;

pub struct State {
    children_type_ids: Vec<TypeId>,
    children: Vec<State>,
    active_child_idx: Option<usize>,
    custom_state: Box<dyn CustomStateTrait>,
}

impl State {
    pub fn new(custom_state: Box<dyn CustomStateTrait>) -> State {
        let children: Vec<State> = custom_state.get_children();
        let children_type_ids: Vec<TypeId> = children.iter().map(|s| s.get_custom_state_type_id()).collect();
        let active_child_idx: Option<usize> = if children.is_empty() { None } else { Some(0) };

        Self {
            children,
            active_child_idx,
            children_type_ids,
            custom_state,
        }
    }

    pub fn update(&mut self, context: &mut StatechartUpdateContext) {
        self.custom_state.update(context);

        if let Some(child_idx) = self.active_child_idx {
            if let Some(child) = self.children.get_mut(child_idx) {
                child.update(context);
            }
        }
    }

    pub fn apply_transitions(&mut self, context: StatechartUpdateContext) {
        for transition_custom_state in context.transitions {
            for idx in 0..self.children.len() {
                let type_id: TypeId = self.children_type_ids[idx];

                let raw_custom_state_type_id: TypeId = {
                    let raw_custom_state: &dyn CustomStateTrait = transition_custom_state.as_ref();
                    raw_custom_state.type_id()
                };

                if type_id == raw_custom_state_type_id {
                    self.children.clear();
                    self.children.push(State::new(transition_custom_state));
                    self.active_child_idx = Some(0);
                    break;
                }
            }
        }
    }

    fn get_custom_state_type_id(&self) -> TypeId {
        let raw_custom_state: &dyn CustomStateTrait = self.custom_state.as_ref();
        raw_custom_state.type_id()
    }

    pub(crate) fn get_recursive_active_type_ids(&self) -> Vec<TypeId> {
        if let Some(child_idx) = self.active_child_idx {
            if let Some(child) = self.children.get(child_idx) {
                let mut active_children_type_ids: Vec<TypeId> = child.get_recursive_active_type_ids();
                active_children_type_ids.push(child.get_custom_state_type_id());
                return active_children_type_ids;
            }
        }

        vec![]
    }
}