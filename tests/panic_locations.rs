//! A panic about a mistake in user code is reported at the user's line (`#[track_caller]`), not
//! inside the library. Everything is checked in one test, because the test swaps the
//! process-wide panic hook.
#![allow(non_camel_case_types)]

use statechart::prelude::*;
use std::panic::{self, AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

/// Runs `f`, which must panic; returns the panic's location and message.
fn caught_panic(f: impl FnOnce()) -> (String, String) {
    let seen = Arc::new(Mutex::new((String::new(), String::new())));
    let hook_seen = Arc::clone(&seen);
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let message = match info.payload().downcast_ref::<String>() {
            Some(message) => message.clone(),
            None => info.payload().downcast_ref::<&str>().unwrap().to_string(),
        };
        *hook_seen.lock().unwrap() = (info.location().unwrap().to_string(), message);
    }));
    let result = catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(previous);
    assert!(result.is_err());
    let seen = seen.lock().unwrap();
    seen.clone()
}

/// `Root_A` requests `Root_Ghost`, which `Root` does not list.
mod unlisted {
    use super::*;

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
        fn update(&mut self, ctx: &mut Context<Self>) {
            ctx.transition(Root_Ghost);
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_Ghost;
    impl State for Root_Ghost {
        type Parent = Root;
    }
}

/// `Root` registers `Foo` twice.
mod double_trigger {
    use super::*;

    pub struct Foo;
    impl Event for Foo {
        type Output = ();
    }

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Foo>().trigger::<Foo>();
        }
    }
    impl Trigger<Foo> for Root {
        fn handle(&mut self, _: &Foo, _: &mut Context<Self>) {}
    }
}

/// `Root` lists `Root_A` twice (two regions), `Other` lists `Other_A` twice in one region.
mod duplicate {
    use super::*;

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>();
            def.region::<Root_A>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
    }

    #[derive(Debug)]
    pub struct Other;
    impl State for Other {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Other_A>()
                .child_with(|_: &Other| Other_B)
                .child_explicit::<Other_A>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Other_A;
    impl State for Other_A {
        type Parent = Other;
    }

    #[derive(Debug)]
    pub struct Other_B;
    impl State for Other_B {
        type Parent = Other;
    }
}

/// `Root` registers the composite `Root_B` with `child_explicit`.
mod explicit_composite {
    use super::*;

    #[derive(Debug)]
    pub struct Root;
    impl State for Root {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_A>().child_explicit::<Root_B>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_A;
    impl State for Root_A {
        type Parent = Root;
    }

    #[derive(Debug)]
    pub struct Root_B;
    impl State for Root_B {
        type Parent = Root;
        fn define(def: &mut Definition<Self>) {
            def.region::<Root_B_X>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Root_B_X;
    impl State for Root_B_X {
        type Parent = Root_B;
    }
}

#[test]
fn panics_about_mistakes_in_user_code_point_at_the_users_line() {
    let here = file!();

    let (location, message) = caught_panic(|| Statechart::new(unlisted::Root).update());
    assert!(location.starts_with(here), "{location}: {message}");
    assert!(message.starts_with("Transition target"), "{message}");

    let (location, message) = caught_panic(|| {
        Statechart::new(double_trigger::Root);
    });
    assert!(location.starts_with(here), "{location}: {message}");
    assert!(message.contains("registers the trigger"), "{message}");

    let (location, message) = caught_panic(|| {
        Statechart::new(duplicate::Root);
    });
    assert!(location.starts_with(here), "{location}: {message}");
    assert_eq!(
        message,
        "State type `panic_locations::duplicate::Root_A` appears more than once in the \
         statechart hierarchy: `panic_locations::duplicate::Root` lists it twice in `define`"
    );

    let (location, message) = caught_panic(|| {
        Statechart::new(duplicate::Other);
    });
    assert!(location.starts_with(here), "{location}: {message}");
    assert!(message.contains("`panic_locations::duplicate::Other` lists it twice"));

    // Whether a child is composite is only known once its own `define` has run, so this panic
    // comes from the library, and its message names the line that registered the child.
    let (location, message) = caught_panic(|| {
        Statechart::new(explicit_composite::Root);
    });
    assert!(message.starts_with("State `panic_locations::explicit_composite::Root_B` has regions"));
    assert!(
        message.contains(&format!("; it is registered at {here}:")),
        "{location}: {message}"
    );
}
