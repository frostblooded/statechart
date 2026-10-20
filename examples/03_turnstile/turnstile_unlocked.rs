use crate::triggers::{Coin, CoinOutcome, Push};
use crate::turnstile::Turnstile;
use crate::turnstile_locked::Turnstile_Locked;
use statechart::prelude::*;

/// Paid: the next push lets one person through.
#[derive(Debug, Default)]
pub struct Turnstile_Unlocked;

impl State for Turnstile_Unlocked {
    type Parent = Turnstile;

    fn define(def: &mut Definition<Self>) {
        def.trigger::<Coin>().trigger::<Push>();
    }
}

impl Trigger<Coin> for Turnstile_Unlocked {
    fn handle(&mut self, _: &Coin, _: &mut Context<Self>) -> CoinOutcome {
        // Already paid: no transition, the coin comes back.
        CoinOutcome::Refunded
    }
}

impl Trigger<Push> for Turnstile_Unlocked {
    fn handle(&mut self, _: &Push, ctx: &mut Context<Self>) {
        ctx.transition(Turnstile_Locked);
    }
}
