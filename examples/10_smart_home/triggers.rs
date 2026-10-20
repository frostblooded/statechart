//! The triggers the smart home reacts to.

use statechart::trigger::Event;

/// The last person leaves the house.
pub struct Leave;
impl Event for Leave {
    type Output = ();
}

/// Someone comes home.
pub struct Arrive;
impl Event for Arrive {
    type Output = ();
}

/// A motion sensor fires.
pub struct Motion;
impl Event for Motion {
    type Output = ();
}

/// Sets the climate target, in °C.
pub struct SetTemperature(pub i32);
impl Event for SetTemperature {
    type Output = ();
}

/// Switches the fan between automatic and always on.
pub struct ToggleFan;
impl Event for ToggleFan {
    type Output = ();
}

/// Describes a subsystem. Each of the three answers; use `trigger_all` to get all the answers.
pub struct Describe;
impl Event for Describe {
    type Output = String;
}

/// Restarts the whole home with its default settings.
pub struct FactoryReset;
impl Event for FactoryReset {
    type Output = ();
}
