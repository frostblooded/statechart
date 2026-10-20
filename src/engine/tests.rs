//! Tests of [`Engine`]'s interface: each operation runs its whole sequence (pending root,
//! traversal, batch) in one call.

use super::{Engine, Sink};
use crate::prelude::*;
use std::any::TypeId;

#[derive(Debug)]
struct Root;
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A>().child::<Root_B>();
    }
}

#[derive(Debug, Default)]
struct Root_A {
    leave_on_update: bool,
}
impl State for Root_A {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Go>();
    }
    fn update(&mut self, ctx: &mut Context<Self>) {
        if self.leave_on_update {
            ctx.transition(Root_B);
        }
    }
}

#[derive(Debug, Default)]
struct Root_B;
impl State for Root_B {
    type Parent = Root;
}

struct Go;
impl Event for Go {
    type Output = u32;
}
impl Trigger<Go> for Root_A {
    fn handle(&mut self, _: &Go, ctx: &mut Context<Self>) -> u32 {
        ctx.transition(Root_B);
        7
    }
}

fn is_live<T: 'static>(engine: &Engine) -> bool {
    engine.is_live(TypeId::of::<T>())
}

#[test]
fn new_enters_the_root_and_its_initial_children() {
    let engine = Engine::new(Root);
    assert_eq!(format!("{engine:?}"), "Root -> Root_A { leave_on_update: false }");
    assert!(is_live::<Root>(&engine));
    assert!(is_live::<Root_A>(&engine));
    assert!(!is_live::<Root_B>(&engine));
}

#[test]
fn update_applies_the_requested_transitions() {
    let mut engine = Engine::new(Root);
    engine.update();
    assert!(is_live::<Root_A>(&engine), "nothing was requested");

    engine
        .state_mut(TypeId::of::<Root_A>())
        .and_then(|s| s.as_any_mut().downcast_mut::<Root_A>())
        .unwrap()
        .leave_on_update = true;
    engine.update();
    assert!(!is_live::<Root_A>(&engine));
    assert!(is_live::<Root_B>(&engine));
}

#[test]
fn dispatch_fills_the_sink_and_applies_the_requested_transitions() {
    let mut engine = Engine::new(Root);
    let mut sink = Sink::First(None);
    engine.dispatch(&Go, &mut sink);
    assert!(matches!(sink, Sink::First(Some(7))));
    assert!(is_live::<Root_B>(&engine));

    let mut sink = Sink::All(Vec::new());
    engine.dispatch(&Go, &mut sink);
    assert!(matches!(sink, Sink::All(ref outputs) if outputs.is_empty()));
}

#[test]
fn queries_see_the_active_configuration() {
    let engine = Engine::new(Root);
    assert!(engine.root().as_any().is::<Root>());
    assert!(engine.state(TypeId::of::<Root_A>()).is_some());
    assert!(engine.state(TypeId::of::<Root_B>()).is_none());
    assert_eq!(
        engine.active_descendant_types(),
        [TypeId::of::<Root_A>()]
    );
}
