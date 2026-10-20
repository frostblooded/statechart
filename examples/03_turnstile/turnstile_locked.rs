use crate::triggers::{Coin, CoinOutcome};
use crate::turnstile::Turnstile;
use crate::turnstile_unlocked::Turnstile_Unlocked;
use statechart::prelude::*;

/// The arm is locked. Only a coin helps; `Push` is not handled here.
#[derive(Debug, Default)]
pub struct Turnstile_Locked;

impl State for Turnstile_Locked {
    type Parent = Turnstile;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Coin>();
    }
}

impl Trigger<Coin> for Turnstile_Locked {
    // The return type is `Coin::Output`; `chart.trigger(Coin)` returns it as `Ok(..)`.
    fn handle(&mut self, _: &Coin, ctx: &mut Context<Self>) -> CoinOutcome {
        ctx.transition(Turnstile_Unlocked);
        CoinOutcome::Unlocked
    }
}
