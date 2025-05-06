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

struct TimerStateChart {
    root: Timer,
}

impl TimerStateChart {
    fn new() -> Self {
        TimerStateChart {
            root: Timer::new()
        }
    }

    fn update(&mut self) {
        let mut context: StatechartUpdateContext = StatechartUpdateContext::new();
        self.root.update(&mut context);
        self.handle_transitions(context);
    }

    fn handle_transitions(&mut self, context: StatechartUpdateContext) {
        for transition_id in context.transitions {
            for idx in 0..self.root.meta_data.children.len() {
                let type_id: TypeId = self.root.meta_data.children_type_ids[idx];

                if type_id == transition_id {
                    self.root.meta_data.active_children_idx = vec![idx];
                }
            }
        }
    }
}

struct TimerMetaData {
    children_type_ids: Vec<TypeId>,
    children: Vec<Box<dyn StateBehavior>>,
    active_children_idx: Vec<usize>
}

impl TimerMetaData {
    fn new() -> Self {
        let default_state: Box<Timer_Running> = Box::new(Timer_Running::new());

        let children: Vec<Box<dyn StateBehavior>> = vec![
            default_state,
            Box::new(Timer_Elapsed::new()),
        ];

        let children_type_ids: Vec<TypeId> = vec![
            TypeId::of::<Timer_Running>(),
            TypeId::of::<Timer_Elapsed>(),
        ];

        TimerMetaData { children, active_children_idx: vec![0], children_type_ids }
    }
}

trait StateBehavior {
    fn update(&mut self, _context: &mut StatechartUpdateContext) {}

    fn transition<NewStateType: 'static>(&self, context: &mut StatechartUpdateContext) where Self: Sized {
        let new_state_type_id = TypeId::of::<NewStateType>();
        context.transitions.push(new_state_type_id);
    }
}

#[allow(non_camel_case_types)]
struct Timer_Running {
    duration: Duration,
    start_time: Instant,
}

impl Timer_Running {
    fn new() -> Self {
        Timer_Running { duration: Duration::from_secs(3), start_time: Instant::now() }
    }
}

impl StateBehavior for Timer_Running {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        let now: Instant = Instant::now();
        let elapsed_time: Duration = now.duration_since(self.start_time);

        if elapsed_time >= self.duration {
            self.transition::<Timer_Elapsed>(context);
        }
    }
}

#[allow(non_camel_case_types)]
struct Timer_Elapsed {}

impl Timer_Elapsed {
    fn new() -> Self {
        Timer_Elapsed {}
    }
}

impl StateBehavior for Timer_Elapsed {}

struct Timer {
    meta_data: TimerMetaData,
}

impl Timer {
    fn new() -> Self {
        Timer { meta_data: TimerMetaData::new() }
    }
}

impl StateBehavior for Timer {
    fn update(&mut self, context: &mut StatechartUpdateContext) {
        for child_idx in &self.meta_data.active_children_idx {
            self.meta_data.children[*child_idx].update(context);
        }
    }
}

fn main() {
    let mut statechart: TimerStateChart = TimerStateChart::new();

    loop {
        thread::sleep(Duration::from_secs(1));
        statechart.update();
        let root: &Timer = &statechart.root;
        println!("Active states: {:?}", root.meta_data.active_children_idx)
    }
}
