//! Data access (spec items 8, 9 and 15): the typed `parent()` / `parent_mut()` and the by-type
//! `ancestor()` / `ancestor_mut()`, from every kind of hook.
#![allow(non_camel_case_types)]

use statechart::prelude::*;

// Root { counter, log }
// └── region: Root_Mid { hits } (initial)
//     Root_Mid
//     ├── region 0: Root_Mid_Left (initial)
//     │   Root_Mid_Left
//     │   └── region: Root_Mid_Left_Deep (initial)
//     └── region 1: Root_Mid_Right (initial)

/// Handled by `Root_Mid_Left`: bumps the root's counter and returns it.
struct Bump;
impl Event for Bump {
    type Output = u32;
}

/// Handled by `Root`: re-enters `Root_Mid` (exits and enters the whole subtree).
struct Restart;
impl Event for Restart {
    type Output = ();
}

/// Handled by `Root_Mid_Left_Deep`: what the ancestor lookups return from there.
struct Probe;
impl Event for Probe {
    type Output = (u32, bool, bool, bool);
}

#[derive(Debug, Default)]
struct Root {
    counter: u32,
    log: Vec<String>,
}
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Mid>();
        def.trigger::<Restart>();
    }
}
impl Trigger<Restart> for Root {
    fn handle(&mut self, _: &Restart, ctx: &mut Context<Self>) {
        ctx.transition(Root_Mid::default());
    }
}

#[derive(Debug, Default)]
struct Root_Mid {
    hits: u32,
}
impl State for Root_Mid {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Mid_Left>();
        def.region::<Root_Mid_Right>();
    }
}

#[derive(Debug, Default)]
struct Root_Mid_Left;
impl State for Root_Mid_Left {
    type Parent = Root_Mid;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_Mid_Left_Deep>();
        def.trigger::<Bump>();
    }

    fn on_enter(&mut self, ctx: &mut Context<Self>) {
        // Typed parent: `&mut Root_Mid`, no Option.
        ctx.parent_mut().hits += 1;
        // Any depth, by type.
        ctx.ancestor_mut::<Root>()
            .unwrap()
            .log
            .push("enter Left".into());
    }

    fn update(&mut self, ctx: &mut Context<Self>) {
        ctx.parent_mut().hits += 1;
        ctx.ancestor_mut::<Root>().unwrap().counter += 10;
    }

    fn on_exit(&mut self, ctx: &mut ExitContext<Self>) {
        // Ancestors are still active (and mutable) while a state is exited.
        ctx.parent_mut().hits += 100;
        ctx.ancestor_mut::<Root>()
            .unwrap()
            .log
            .push("exit Left".into());
    }
}
impl Trigger<Bump> for Root_Mid_Left {
    fn handle(&mut self, _: &Bump, ctx: &mut Context<Self>) -> u32 {
        let root = ctx.ancestor_mut::<Root>().unwrap();
        root.counter += 1;
        root.counter
    }
}

#[derive(Debug, Default)]
struct Root_Mid_Left_Deep;
impl State for Root_Mid_Left_Deep {
    type Parent = Root_Mid_Left;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Probe>();
    }
}
impl Trigger<Probe> for Root_Mid_Left_Deep {
    fn handle(&mut self, _: &Probe, ctx: &mut Context<Self>) -> (u32, bool, bool, bool) {
        (
            // Great-grandparent and grandparent, by type.
            ctx.ancestor::<Root>().unwrap().counter + ctx.ancestor::<Root_Mid>().unwrap().hits,
            // A state in another region is not an ancestor.
            ctx.ancestor::<Root_Mid_Right>().is_none(),
            // Neither is the state itself.
            ctx.ancestor::<Root_Mid_Left_Deep>().is_none(),
            // `parent()` is the same state as `ancestor::<Parent>()`.
            std::ptr::eq(ctx.parent(), ctx.ancestor::<Root_Mid_Left>().unwrap()),
        )
    }
}

#[derive(Debug, Default)]
struct Root_Mid_Right;
impl State for Root_Mid_Right {
    type Parent = Root_Mid;
    fn update(&mut self, ctx: &mut Context<Self>) {
        // The parent is shared with the other region; both change it in turn.
        ctx.parent_mut().hits += 1;
        ctx.ancestor_mut::<Root>().unwrap().counter += 1;
    }
}

#[test]
fn on_enter_reads_and_changes_the_parent_and_ancestors() {
    let chart = Statechart::new(Root::default());
    assert_eq!(chart.state::<Root_Mid>().unwrap().hits, 1);
    assert_eq!(chart.root().log, ["enter Left"]);
}

#[test]
fn update_changes_the_parent_and_ancestors_from_every_region() {
    let mut chart = Statechart::new(Root::default());
    chart.update();
    // Left: +1 hits, +10 counter. Right: +1 hits, +1 counter.
    assert_eq!(chart.state::<Root_Mid>().unwrap().hits, 3);
    assert_eq!(chart.root().counter, 11);
}

#[test]
fn a_trigger_handler_changes_an_ancestor_and_returns_a_typed_output() {
    let mut chart = Statechart::new(Root::default());
    assert_eq!(chart.trigger(Bump), Ok(1));
    assert_eq!(chart.trigger(Bump), Ok(2));
    assert_eq!(chart.root().counter, 2);
}

#[test]
fn on_exit_changes_ancestors_that_are_still_active() {
    let mut chart = Statechart::new(Root::default());
    chart.trigger(Restart).unwrap();
    // The old Root_Mid got +100 from Left's on_exit, then was replaced by a fresh one that got
    // +1 from the new Left's on_enter.
    assert_eq!(chart.state::<Root_Mid>().unwrap().hits, 1);
    assert_eq!(chart.root().log, ["enter Left", "exit Left", "enter Left"]);
}

#[test]
fn ancestor_lookup_works_at_any_depth_and_excludes_self_and_other_regions() {
    let mut chart = Statechart::new(Root::default());
    chart.root_mut().counter = 40;
    assert_eq!(chart.trigger(Probe), Ok((41, true, true, true)));
}

#[test]
fn root_data_is_reachable_from_outside_the_chart() {
    let mut chart = Statechart::new(Root::default());
    chart.root_mut().counter = 41;
    assert_eq!(chart.trigger(Bump), Ok(42));
    assert_eq!(chart.root().counter, 42);
}
