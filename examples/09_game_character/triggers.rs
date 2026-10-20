//! The triggers the character reacts to.

use statechart::trigger::Event;

/// Starts walking. Answers whether the character could.
pub struct Walk;
impl Event for Walk {
    type Output = bool;
}

/// Stops walking.
pub struct Stop;
impl Event for Stop {
    type Output = ();
}

/// Jumps, from standing or walking.
pub struct Jump;
impl Event for Jump {
    type Output = ();
}

/// Attacks with the current weapon. Answers the damage dealt.
pub struct Attack;
impl Event for Attack {
    type Output = u32;
}

/// Picks up a sword.
pub struct EquipSword;
impl Event for EquipSword {
    type Output = ();
}

/// Picks up a bow with some arrows.
pub struct EquipBow {
    pub arrows: u32,
}
impl Event for EquipBow {
    type Output = ();
}

/// Takes some health away.
pub struct Damage(pub i32);
impl Event for Damage {
    type Output = ();
}

/// Poisons a healthy character.
pub struct Poison;
impl Event for Poison {
    type Output = ();
}

/// Brings a dead character back.
pub struct Revive;
impl Event for Revive {
    type Output = ();
}

/// Describes the character. Every region answers; use `trigger_all` to get all the answers.
pub struct Describe;
impl Event for Describe {
    type Output = String;
}
