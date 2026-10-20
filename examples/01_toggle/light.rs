use crate::light_off::Light_Off;
use crate::light_on::Light_On;
use statechart::prelude::*;

/// The root. It has no data and handles no triggers: it only lists its children.
#[derive(Debug, Default)]
pub struct Light;

impl State for Light {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        // One region: Light_Off is active first, Light_On is the other child.
        def.region::<Light_Off>().child::<Light_On>();
    }
}
