use std::any::TypeId;
use crate::custom_state_trait::CustomStateTrait;
use crate::state_meta_data::StateMetaData;
use crate::state_trait::StateTrait;
use crate::statechart_update_context::StatechartUpdateContext;

pub struct State {
    pub(crate) meta_data: StateMetaData,
    custom_state: Box<dyn CustomStateTrait>,
}

impl State {
    pub fn handle_transitions(&mut self, context: StatechartUpdateContext) {
        for transition_id in context.transitions {
            for idx in 0..self.meta_data.children.len() {
                let type_id: TypeId = self.meta_data.children_type_ids[idx];

                if type_id == transition_id {
                    self.meta_data.active_children_idx = vec![idx];
                }
            }
        }
    }
}

impl StateTrait for State {
    fn new<T: CustomStateTrait + 'static>() -> State {
        let custom_state: Box<T> = Box::new(T::new());
        let children: Vec<Box<dyn StateTrait>> = T::get_children();

        Self {
            meta_data: StateMetaData::new(children),
            custom_state,
        }
    }

    fn update(&self, context: &mut StatechartUpdateContext) {
        self.meta_data.update(context);
    }
}
