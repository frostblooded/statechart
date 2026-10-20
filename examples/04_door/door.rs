use crate::door_closed::Door_Closed;
use crate::door_locked::Door_Locked;
use crate::door_open::Door_Open;
use statechart::prelude::*;
use std::fmt;

/// The root. Its `log` collects what the children's hooks write.
#[derive(Default)]
pub struct Door {
    pub log: Vec<String>,
}

// Only the name: the log would make the active configuration hard to read.
impl fmt::Debug for Door {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Door")
    }
}

impl State for Door {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<Door_Closed>()
            .child::<Door_Open>()
            // No constructor: Door_Locked is only entered by `ctx.transition(Door_Locked { code })`,
            // because its code only makes sense when someone gives it.
            .child_explicit::<Door_Locked>();
    }
}
