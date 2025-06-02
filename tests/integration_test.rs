#![allow(non_camel_case_types)]

mod test_1
{
    use std::any::TypeId;
    use statechart::custom_state_trait::CustomStateTrait;
    use statechart::statechart::Statechart;
    use statechart::statechart_update_context::StatechartUpdateContext;

    #[derive(Debug, Default)]
    struct A {}

    impl CustomStateTrait for A {
        fn get_possible_children_type_ids(&self) -> Vec<TypeId> {
            vec![
                TypeId::of::<A_A>(),
                TypeId::of::<A_B>(),
            ]
        }

        fn get_initial_child(&self) -> Option<Box<dyn CustomStateTrait>> {
            Some(Box::new(A_A::default()))
        }
    }

    #[derive(Debug, Default)]
    struct A_A {}

    impl CustomStateTrait for A_A {
        fn update(&mut self, context: &mut StatechartUpdateContext) {
            context.transition(Box::new(A_B::default()));
        }
    }

    #[derive(Debug, Default)]
    struct A_B {}

    impl CustomStateTrait for A_B {}

    #[test]
    fn test_basic_transition() {
        let mut statechart = Statechart::new(Box::new(A::default()));
        statechart.update();
        assert_eq!(statechart.get_recursive_active_type_ids(), vec![TypeId::of::<A_B>()]);
    }

    #[test]
    fn test_debug_formatting() {
        let mut statechart = Statechart::new(Box::new(A::default()));
        assert_eq!(statechart.get_debug_name(), "A -> A_A");
        statechart.update();
        assert_eq!(statechart.get_debug_name(), "A -> A_B");
    }
}

mod test_2 {
    use statechart::custom_state_trait::CustomStateTrait;
    use statechart::statechart::Statechart;

    #[derive(Debug, Default)]
    struct A {}

    impl CustomStateTrait for A {
        fn get_initial_child(&self) -> Option<Box<dyn CustomStateTrait>> {
            Some(Box::new(B::default()))
        }
    }

    #[derive(Debug, Default)]
    struct B {}

    impl CustomStateTrait for B {}

    #[test]
    #[should_panic(expected = "Initial child of state isn't a possible child for it")]
    fn test_asserts_if_initial_child_is_not_one_of_the_possible_children() {
        Statechart::new(Box::new(A::default()));
    }
}
