//! User decision D3: every public item is reachable through its module path
//! (`statechart::state::State`, `statechart::statechart::Statechart`, ...), and
//! `statechart::prelude` re-exports the everyday names of those same items. The crate root
//! re-exports nothing but the modules.
#![allow(non_camel_case_types)]

/// Written with full module paths only, never the prelude.
mod by_module_path {
    use statechart::context::{Context, ExitContext};
    use statechart::definition::Definition;
    use statechart::state::{NoParent, State};
    use statechart::statechart::Statechart;
    use statechart::trigger::{Event, NotHandled, Trigger};

    pub struct Ask;
    impl Event for Ask {
        type Output = u8;
    }

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_Leaf>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_Leaf;
    impl State for Root_Leaf {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Ask>();
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {}
    }
    impl Trigger<Ask> for Root_Leaf {
        fn handle(&mut self, _: &Ask, _: &mut Context<Self>) -> u8 {
            7
        }
    }

    pub fn run() -> (String, Result<u8, NotHandled>) {
        let mut chart = Statechart::new(Root);
        let output = chart.trigger(Ask);
        (chart.get_debug_name(), output)
    }
}

#[test]
fn the_api_works_through_module_paths_without_the_prelude() {
    let (name, output) = by_module_path::run();
    assert_eq!(name, "Root -> Root_Leaf");
    assert_eq!(output, Ok(7));
}

/// Compiles only if the prelude's traits are the very traits the module-path fixture implements.
fn prelude_state<S: statechart::prelude::State>() {}
fn prelude_event<E: statechart::prelude::Event>() {}
fn prelude_trigger<S: statechart::prelude::Trigger<E>, E: statechart::prelude::Event>() {}

#[test]
fn the_prelude_re_exports_the_same_items_as_the_module_paths() {
    use by_module_path::{Ask, Root, Root_Leaf};
    use std::any::TypeId;

    prelude_state::<Root>();
    prelude_state::<Root_Leaf>();
    prelude_event::<Ask>();
    prelude_trigger::<Root_Leaf, Ask>();

    assert_eq!(
        TypeId::of::<statechart::prelude::Statechart<Root>>(),
        TypeId::of::<statechart::statechart::Statechart<Root>>()
    );
    assert_eq!(
        TypeId::of::<statechart::prelude::Definition<Root>>(),
        TypeId::of::<statechart::definition::Definition<Root>>()
    );
    assert_eq!(
        TypeId::of::<statechart::prelude::Context<'static, Root>>(),
        TypeId::of::<statechart::context::Context<'static, Root>>()
    );
    assert_eq!(
        TypeId::of::<statechart::prelude::ExitContext<'static, Root>>(),
        TypeId::of::<statechart::context::ExitContext<'static, Root>>()
    );
    assert_eq!(
        TypeId::of::<statechart::prelude::NoParent>(),
        TypeId::of::<statechart::state::NoParent>()
    );
    assert_eq!(
        TypeId::of::<statechart::prelude::NotHandled>(),
        TypeId::of::<statechart::trigger::NotHandled>()
    );
}
