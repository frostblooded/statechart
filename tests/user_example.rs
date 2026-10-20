//! The example from the original request: `A` has children `B1` and `B2`; `B1` implements the
//! trigger `Foo`, `B2` does not. Named as breadcrumbs: `A_B1` is `B1`, a child of `A`.
#![allow(non_camel_case_types)]

use statechart::prelude::*;

/// The trigger from the request. Its output says who handled it.
struct Foo;
impl Event for Foo {
    type Output = &'static str;
}

/// Switches `A`'s region to `A_B2`.
struct SwitchToB2;
impl Event for SwitchToB2 {
    type Output = ();
}

/// `A` does not implement `Foo`.
mod a_without_foo {
    use super::*;

    #[derive(Debug)]
    pub struct A;
    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_B1>().child::<A_B2>();
            def.trigger::<SwitchToB2>();
        }
    }
    impl Trigger<SwitchToB2> for A {
        fn handle(&mut self, _: &SwitchToB2, ctx: &mut Context<Self>) {
            ctx.transition(A_B2);
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B1;
    impl State for A_B1 {
        type Parent = A;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Foo>();
        }
    }
    impl Trigger<Foo> for A_B1 {
        fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
            "A_B1"
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B2;
    impl State for A_B2 {
        type Parent = A;
    }
}

/// Same chart, but `A` implements `Foo` (and counts how often it ran), so `A_B2` falls back
/// to it.
mod a_with_foo {
    use super::*;

    #[derive(Debug, Default)]
    pub struct A {
        pub foo_calls: u32,
    }
    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_B1>().child::<A_B2>();
            def.trigger::<SwitchToB2>().trigger::<Foo>();
        }
    }
    impl Trigger<SwitchToB2> for A {
        fn handle(&mut self, _: &SwitchToB2, ctx: &mut Context<Self>) {
            ctx.transition(A_B2);
        }
    }
    impl Trigger<Foo> for A {
        fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
            self.foo_calls += 1;
            "A"
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B1;
    impl State for A_B1 {
        type Parent = A;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Foo>();
        }
    }
    impl Trigger<Foo> for A_B1 {
        fn handle(&mut self, _: &Foo, _: &mut Context<Self>) -> &'static str {
            "A_B1"
        }
    }

    #[derive(Debug, Default)]
    pub struct A_B2;
    impl State for A_B2 {
        type Parent = A;
    }
}

#[test]
fn b1_active_runs_only_b1s_foo() {
    let mut chart = Statechart::new(a_with_foo::A::default());
    assert!(chart.is_active::<a_with_foo::A_B1>());
    assert_eq!(chart.trigger(Foo), Ok("A_B1"));
    assert_eq!(chart.trigger_all(Foo), Ok(vec!["A_B1"]));
    // A implements Foo too, but only the deepest handler runs.
    assert_eq!(chart.root().foo_calls, 0);
}

#[test]
fn b2_active_and_a_without_foo_is_not_handled() {
    let mut chart = Statechart::new(a_without_foo::A);
    assert_eq!(chart.trigger(Foo), Ok("A_B1"));
    chart.trigger(SwitchToB2).unwrap();
    assert_eq!(chart.get_debug_name(), "A -> A_B2");
    assert_eq!(chart.trigger(Foo), Err(NotHandled));
    assert_eq!(chart.trigger_all(Foo), Err(NotHandled));
}

#[test]
fn b2_active_and_a_with_foo_falls_back_to_a() {
    let mut chart = Statechart::new(a_with_foo::A::default());
    chart.trigger(SwitchToB2).unwrap();
    assert!(chart.is_active::<a_with_foo::A_B2>());
    assert_eq!(chart.trigger(Foo), Ok("A"));
    assert_eq!(chart.root().foo_calls, 1);
}
