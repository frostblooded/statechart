use crate::custom_state_trait::CustomStateTrait;
use crate::state_meta_data::StateMetaData;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::TypeId;

pub struct State {
    pub(crate) meta_data: StateMetaData,
    custom_state: Box<dyn CustomStateTrait>,
}

impl State {
    pub fn new<T: CustomStateTrait + 'static>() -> State {
        let custom_state: Box<T> = Box::new(T::new());
        let children: Vec<State> = T::get_children();
        let children_type_ids: Vec<TypeId> = children.iter().map(|s| s.get_custom_state_type_id()).collect();

        Self {
            meta_data: StateMetaData::new(children, children_type_ids),
            custom_state,
        }
    }

    pub fn update(&mut self, context: &mut StatechartUpdateContext) {
        self.meta_data.update(context);
        self.custom_state.update(context);
    }

    pub fn handle_transitions(&mut self, context: StatechartUpdateContext) {
        for transition_id in context.transitions {
            for idx in 0..self.meta_data.children.len() {
                let type_id: TypeId = self.meta_data.children_type_ids[idx];

                if type_id == transition_id {
                    self.meta_data.active_children_idx = Some(idx);
                }
            }
        }
    }
    
    pub fn get_custom_state_type_id(&self) -> TypeId {
        self.custom_state.type_id()
    }
}
