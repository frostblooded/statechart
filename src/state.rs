use crate::custom_state_trait::CustomStateTrait;
use crate::statechart_update_context::StatechartUpdateContext;
use std::any::TypeId;
use std::fmt::Debug;
use std::ops::DerefMut;

pub struct State {
    possible_children_type_ids: Vec<TypeId>,
    custom_state: Box<dyn CustomStateTrait>,
    child: Option<Box<State>>,
}

impl State {
    pub fn new(custom_state: Box<dyn CustomStateTrait>) -> State {
        let possible_children_type_ids: Vec<TypeId> = custom_state.get_possible_children_type_ids();
        let child: Option<Box<State>> = custom_state.get_initial_child().map(State::new).map(Box::new);

        if let Some(c) = &child {
            let child_type_id: TypeId = c.get_custom_state_type_id();
            debug_assert!(possible_children_type_ids.contains(&child_type_id), "Initial child of state isn't a possible child for it");
        }

        Self {
            possible_children_type_ids,
            custom_state,
            child
        }
    }

    pub fn on_enter(&mut self) {
        self.custom_state.on_enter();
    }

    pub fn on_exit(&mut self) {
        self.custom_state.on_exit();
    }

    pub fn update(&mut self, context: &mut StatechartUpdateContext) {
        self.custom_state.update(context);

        if let Some(child) = &mut self.child {
            child.deref_mut().update(context);
        }
    }

    pub fn apply_transitions(&mut self, context: StatechartUpdateContext) {
        // TODO: Properly handle trans-level transitions.
        for mut transition_custom_state in context.transitions {
            for child_type_id in &self.possible_children_type_ids {
                let raw_custom_state_type_id: TypeId = transition_custom_state.as_ref().type_id();

                if *child_type_id == raw_custom_state_type_id {
                    if let Some(c) = &mut self.child {
                        c.on_exit();
                    }

                    transition_custom_state.on_enter();
                    self.child = Some(Box::new(State::new(transition_custom_state)));
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
        if let Some(child) = &self.child {
            let mut active_children_type_ids: Vec<TypeId> = child.get_recursive_active_type_ids();
            active_children_type_ids.push(child.get_custom_state_type_id());
            return active_children_type_ids;
        }

        vec![]
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(child) = &self.child {
            write!(f, "{:?} -> {:?}", self.custom_state, child)
        } else {
            write!(f, "{:?}", self.custom_state)
        }
    }
}
