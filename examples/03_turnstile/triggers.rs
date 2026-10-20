//! The triggers the turnstile reacts to.

use statechart::trigger::Event;

/// Drops a coin in the slot. Answers what happened to it.
pub struct Coin;
impl Event for Coin {
    type Output = CoinOutcome;
}

/// What the turnstile did with a coin.
#[derive(Debug, PartialEq, Eq)]
pub enum CoinOutcome {
    /// The coin was taken and the arm unlocked.
    Unlocked,
    /// The arm was already unlocked, so the coin was given back.
    Refunded,
}

/// Pushes the arm. Only an unlocked turnstile handles it.
pub struct Push;
impl Event for Push {
    type Output = ();
}
