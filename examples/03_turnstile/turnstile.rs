use crate::turnstile_locked::Turnstile_Locked;
use crate::turnstile_unlocked::Turnstile_Unlocked;
use statechart::prelude::*;

/// The root: a coin-operated turnstile.
#[derive(Debug, Default)]
pub struct Turnstile;

impl State for Turnstile {
    type Parent = NoParent;

    fn define(def: &mut Definition<Self>) {
        def.region::<Turnstile_Locked>()
            .child::<Turnstile_Unlocked>();
    }
}
