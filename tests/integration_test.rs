//! The behaviours of the original test suite, ported to the new API. Module names are kept so
//! that `cargo test test_3::` still selects one module.
#![allow(non_camel_case_types)]

mod test_1 {
    use statechart::prelude::*;
    use std::any::TypeId;

    #[derive(Debug)]
    struct A;

    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_A>().child::<A_B>();
        }
    }

    #[derive(Debug, Default)]
    struct A_A;

    impl State for A_A {
        type Parent = A;
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(A_B);
        }
    }

    #[derive(Debug, Default)]
    struct A_B;

    impl State for A_B {
        type Parent = A;
    }

    #[test]
    fn test_basic_transition() {
        let mut statechart = Statechart::new(A);
        statechart.update();
        assert_eq!(
            statechart.get_recursive_active_type_ids(),
            vec![TypeId::of::<A_B>()]
        );
    }

    #[test]
    fn test_debug_formatting() {
        let mut statechart = Statechart::new(A);
        assert_eq!(statechart.get_debug_name(), "A -> A_A");
        statechart.update();
        assert_eq!(statechart.get_debug_name(), "A -> A_B");
    }
}

/// The old `"Initial child of state isn't a possible child for it"` panic cannot happen any
/// more: a region is declared by naming its initial child, and a child whose `Parent` is not
/// the declaring state does not compile (`compile_fail` doctest on `Definition::region`).
/// The structural mistakes the compiler cannot see still panic, with these exact messages.
///
/// The duplicate-state check is an `assert!`, so it fires in debug *and* release builds
/// (user decision D1): these tests are deliberately not gated on `cfg(debug_assertions)`.
mod test_2 {
    pub mod duplicate {
        use statechart::prelude::*;

        #[derive(Debug)]
        pub struct A;
        impl State for A {
            type Parent = NoParent;
            fn define(def: &mut Definition<Self>) {
                // The same state type in two regions.
                def.region::<A_A>();
                def.region::<A_A>();
            }
        }

        #[derive(Debug, Default)]
        pub struct A_A;
        impl State for A_A {
            type Parent = A;
        }
    }

    #[test]
    #[should_panic(
        expected = "State type `integration_test::test_2::duplicate::A_A` appears more than once in the statechart hierarchy"
    )]
    fn test_asserts_if_a_state_type_appears_twice() {
        statechart::statechart::Statechart::new(duplicate::A);
    }

    pub mod duplicate_in_one_region {
        use statechart::prelude::*;

        #[derive(Debug)]
        pub struct A;
        impl State for A {
            type Parent = NoParent;
            fn define(def: &mut Definition<Self>) {
                // The initial child listed again as a non-initial child of the same region.
                def.region::<A_A>().child::<A_B>().child::<A_A>();
            }
        }

        #[derive(Debug, Default)]
        pub struct A_A;
        impl State for A_A {
            type Parent = A;
        }

        #[derive(Debug, Default)]
        pub struct A_B;
        impl State for A_B {
            type Parent = A;
        }
    }

    #[test]
    #[should_panic(
        expected = "State type `integration_test::test_2::duplicate_in_one_region::A_A` appears more than once in the statechart hierarchy"
    )]
    fn test_asserts_if_a_state_type_is_listed_twice_in_one_region() {
        statechart::statechart::Statechart::new(duplicate_in_one_region::A);
    }

    pub mod unlisted {
        use statechart::prelude::*;

        #[derive(Debug)]
        pub struct A;
        impl State for A {
            type Parent = NoParent;
            fn define(def: &mut Definition<Self>) {
                def.region::<A_A>();
            }
        }

        #[derive(Debug, Default)]
        pub struct A_A;
        impl State for A_A {
            type Parent = A;
            fn update(&mut self, ctx: &mut Context<Self>) {
                ctx.transition(A_Ghost);
            }
        }

        /// Names `A` as its parent, but `A::define` does not list it.
        #[derive(Debug, Default)]
        pub struct A_Ghost;
        impl State for A_Ghost {
            type Parent = A;
        }
    }

    #[test]
    #[should_panic(
        expected = "Transition target `integration_test::test_2::unlisted::A_Ghost` is not part of this statechart: its parent `integration_test::test_2::unlisted::A` does not list it in `define`"
    )]
    fn test_panics_if_a_transition_target_is_not_listed_by_its_parent() {
        let mut statechart = statechart::statechart::Statechart::new(unlisted::A);
        statechart.update();
    }
}

mod test_3 {
    use statechart::prelude::*;
    use std::fmt::{Debug, Formatter};

    struct A {
        on_enter_called: bool,
    }

    impl A {
        fn construct() -> Self {
            Self {
                on_enter_called: false,
            }
        }
    }

    impl State for A {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<A_A>().child_explicit::<A_B>();
        }
        fn on_enter(&mut self, _: &mut Context<Self>) {
            self.on_enter_called = true;
        }
    }

    impl Debug for A {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "A({})", self.on_enter_called)
        }
    }

    #[derive(Default)]
    struct A_A {
        on_enter_called: bool,
    }

    impl State for A_A {
        type Parent = A;
        fn on_enter(&mut self, _: &mut Context<Self>) {
            self.on_enter_called = true;
        }
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(A_B::construct());
        }
    }

    impl Debug for A_A {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "A_A({})", self.on_enter_called)
        }
    }

    struct A_B {
        inner_on_enter_called: bool,
    }

    impl A_B {
        fn construct() -> Self {
            Self {
                inner_on_enter_called: false,
            }
        }
    }

    impl State for A_B {
        type Parent = A;
        fn on_enter(&mut self, _: &mut Context<Self>) {
            self.inner_on_enter_called = true;
        }
    }

    impl Debug for A_B {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "A_B({})", self.inner_on_enter_called)
        }
    }

    /// Initial children get `on_enter` too (the old code skipped them: "A(true) -> A_A").
    #[test]
    fn test_on_enter_called() {
        let sc: Statechart<A> = Statechart::new(A::construct());
        assert_eq!(sc.get_debug_name(), "A(true) -> A_A(true)");
    }

    #[test]
    fn test_inner_on_enter_called() {
        let mut sc: Statechart<A> = Statechart::new(A::construct());
        sc.update();
        assert_eq!(sc.get_debug_name(), "A(true) -> A_B(true)");
    }
}
