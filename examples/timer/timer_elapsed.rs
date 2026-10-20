use crate::timer::Timer;
use statechart::prelude::*;

/// The countdown reached zero. Only `Reset` (handled by the root) leaves this state.
#[derive(Debug, Default)]
pub struct Timer_Elapsed;

impl State for Timer_Elapsed {
    type Parent = Timer;
}
