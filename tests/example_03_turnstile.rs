// Runs the state code from the `03_turnstile` example: triggers that return a value, and
// `Err(NotHandled)` for a trigger the active state does not handle.
#![allow(non_camel_case_types)]

#[path = "../examples/03_turnstile/triggers.rs"]
mod triggers;
#[path = "../examples/03_turnstile/turnstile.rs"]
mod turnstile;
#[path = "../examples/03_turnstile/turnstile_locked.rs"]
mod turnstile_locked;
#[path = "../examples/03_turnstile/turnstile_unlocked.rs"]
mod turnstile_unlocked;

use statechart::prelude::*;
use triggers::{Coin, CoinOutcome, Push};
use turnstile::Turnstile;
use turnstile_locked::Turnstile_Locked;
use turnstile_unlocked::Turnstile_Unlocked;

#[test]
fn test_starts_locked() {
    let chart = Statechart::new(Turnstile);
    assert_eq!(chart.get_debug_name(), "Turnstile -> Turnstile_Locked");
}

#[test]
fn test_pushing_a_locked_turnstile_is_not_handled() {
    let mut chart = Statechart::new(Turnstile);
    assert_eq!(chart.trigger(Push), Err(NotHandled));
    assert!(chart.is_active::<Turnstile_Locked>());
}

#[test]
fn test_a_coin_unlocks_and_a_push_locks_again() {
    let mut chart = Statechart::new(Turnstile);
    assert_eq!(chart.trigger(Coin), Ok(CoinOutcome::Unlocked));
    assert!(chart.is_active::<Turnstile_Unlocked>());
    assert_eq!(chart.trigger(Push), Ok(()));
    assert!(chart.is_active::<Turnstile_Locked>());
}

#[test]
fn test_a_second_coin_is_refunded() {
    let mut chart = Statechart::new(Turnstile);
    chart.trigger(Coin).unwrap();
    assert_eq!(chart.trigger(Coin), Ok(CoinOutcome::Refunded));
    assert!(chart.is_active::<Turnstile_Unlocked>());
}
