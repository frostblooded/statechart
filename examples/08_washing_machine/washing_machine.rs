use crate::washing_machine_done::WashingMachine_Done;
use crate::washing_machine_ready::WashingMachine_Ready;
use crate::washing_machine_running::WashingMachine_Running;
use statechart::prelude::*;

/// The root.
#[derive(Debug, Default)]
pub struct WashingMachine;

impl State for WashingMachine {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<WashingMachine_Ready>()
            .child::<WashingMachine_Running>()
            .child::<WashingMachine_Done>();
    }
}
