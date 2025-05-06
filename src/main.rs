use std::any::TypeId;
use std::thread;
use std::time::{Duration, Instant};

struct StatechartUpdateContext {
    transitions: Vec<TypeId>
}

impl StatechartUpdateContext {
    fn new() -> Self {
        StatechartUpdateContext {
            transitions: vec![]
        }
    }
}

trait StateBehavior {
    fn internal_new() -> Self where Self: Sized;
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}
    fn get_meta_data_mut(&mut self) -> &mut StateMetaData;
    fn get_meta_data(&self) -> &StateMetaData;
    fn init_meta_data(&mut self) {}

    fn new() -> Self where Self: Sized {
        let mut state = Self::internal_new();
        state.init_meta_data();
        state
    }

    fn transition<NewStateType: 'static>(&self, context: &mut StatechartUpdateContext) where Self: Sized {
        let new_state_type_id = TypeId::of::<NewStateType>();
        context.transitions.push(new_state_type_id);
    }

    fn apply_transition(&mut self, context: &StatechartUpdateContext) {
        for transition_id in &context.transitions {
            for idx in 0..self.get_meta_data().children.len() {
                let type_id: TypeId = self.get_meta_data().children_type_ids[idx];

                if type_id == *transition_id {
                    self.get_meta_data_mut().active_child_idx = idx;
                }
            }
        }
    }
}

struct TimerStateChart {
    root: Box<dyn StateBehavior>,
}

impl TimerStateChart {
    fn new() -> Self {
        TimerStateChart {
            root: Box::new(Timer::new())
        }
    }

    fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
        self.handle_transitions(context);
    }

    fn handle_transitions(&mut self, context: StatechartUpdateContext) {
        let root: &mut dyn StateBehavior = &mut *self.root;
        root.apply_transition(&context);
    }
}

struct Timer {
    meta_data: StateMetaData,
}

impl StateBehavior for Timer {
    fn internal_new() -> Self
    where
        Self: Sized
    {
        Self {
            meta_data: StateMetaData::new(),
        }
    }

    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let active_child_idx: usize = self.meta_data.active_child_idx;
        let active_child: &mut Box<dyn StateBehavior> = &mut self.meta_data.children[active_child_idx];
        active_child.update(context);
    }

    fn get_meta_data_mut(&mut self) -> &mut StateMetaData {
        &mut self.meta_data
    }

    fn get_meta_data(&self) -> &StateMetaData {
        &self.meta_data
    }

    fn init_meta_data(&mut self) {
        let children: Vec<Box<dyn StateBehavior>> = vec![
            Box::new(Timer_Running::new()),
            Box::new(Timer_Elapsed::new()),
        ];

        let children_type_ids: Vec<TypeId> = vec![
            TypeId::of::<Timer_Running>(),
            TypeId::of::<Timer_Elapsed>(),
        ];

        self.meta_data = StateMetaData { children, active_child_idx: 0, children_type_ids };
    }
}

struct StateMetaData {
    children_type_ids: Vec<TypeId>,
    children: Vec<Box<dyn StateBehavior>>,
    active_child_idx: usize,
}

impl StateMetaData {
    fn new() -> Self {
        StateMetaData {
            children_type_ids: vec![],
            children: vec![],
            active_child_idx: 0,
        }
    }
}

#[allow(non_camel_case_types)]
struct Timer_Running {
    meta_data: StateMetaData,
    duration: Duration,
    start_time: Instant,
}

impl StateBehavior for Timer_Running {
    fn internal_new() -> Self
    where
        Self: Sized
    {
        Self {
            meta_data: StateMetaData::new(),
            duration: Duration::from_secs(3),
            start_time: Instant::now()
        }
    }

    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let now: Instant = Instant::now();
        let elapsed_time: Duration = now.duration_since(self.start_time);

        if elapsed_time >= self.duration {
            self.transition::<Timer_Elapsed>(context);
        }
    }

    fn get_meta_data_mut(&mut self) -> &mut StateMetaData {
        &mut self.meta_data
    }

    fn get_meta_data(&self) -> &StateMetaData {
        &self.meta_data
    }
}

#[allow(non_camel_case_types)]
struct Timer_Elapsed {
    meta_data: StateMetaData,
}

impl StateBehavior for Timer_Elapsed {
    fn internal_new() -> Self
    where
        Self: Sized
    {
        Self {
            meta_data: StateMetaData::new(),
        }
    }

    fn get_meta_data_mut(&mut self) -> &mut StateMetaData {
        &mut self.meta_data
    }

    fn get_meta_data(&self) -> &StateMetaData {
        &self.meta_data
    }
}

fn main() {
    let mut statechart: TimerStateChart = TimerStateChart::new();

    loop {
        thread::sleep(Duration::from_secs(1));
        statechart.update();
        let root: &dyn StateBehavior = &*statechart.root;
        println!("Active state: {:?}", root.get_meta_data().active_child_idx)
    }
}
