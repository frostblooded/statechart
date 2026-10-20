use crate::traffic_light_green::TrafficLight_Green;
use crate::traffic_light_red::TrafficLight_Red;
use crate::traffic_light_yellow::TrafficLight_Yellow;
use statechart::prelude::*;

/// The root: cycles red, green, yellow, red, ...
#[derive(Debug, Default)]
pub struct TrafficLight;

impl State for TrafficLight {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<TrafficLight_Red>()
            .child::<TrafficLight_Green>()
            .child::<TrafficLight_Yellow>();
    }
}
