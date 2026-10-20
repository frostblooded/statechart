//! The hot paths do not allocate: `update` without transitions, `trigger` (with a non-zero-sized
//! output, with one region and with parallel regions), `is_active` and `state`.
//!
//! A counting global allocator needs `unsafe`; it is confined to this test (the library itself
//! forbids `unsafe`). The counter is per thread, so the test harness's own allocations on other
//! threads are not counted.
#![allow(non_camel_case_types)]

use statechart::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations_in(f: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);
    f();
    ALLOCATIONS.with(Cell::get) - before
}

// Root -> [Root_A, Root_B]; Root_B -> Root_B_X. Root_A and Root_B_X handle `Get`.

struct Get;
impl Event for Get {
    type Output = u32;
}

#[derive(Debug)]
struct Root;
impl State for Root {
    type Parent = NoParent;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_A>();
        def.region::<Root_B>();
    }
}

#[derive(Debug, Default)]
struct Root_A;
impl State for Root_A {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Get>();
    }
}
impl Trigger<Get> for Root_A {
    fn handle(&mut self, _: &Get, _: &mut Context<Self>) -> u32 {
        1
    }
}

#[derive(Debug, Default)]
struct Root_B;
impl State for Root_B {
    type Parent = Root;
    fn define(def: &mut Definition<Self>) {
        def.region::<Root_B_X>();
    }
}

#[derive(Debug, Default)]
struct Root_B_X;
impl State for Root_B_X {
    type Parent = Root_B;
    fn define(def: &mut Definition<Self>) {
        def.trigger::<Get>();
    }
}
impl Trigger<Get> for Root_B_X {
    fn handle(&mut self, _: &Get, _: &mut Context<Self>) -> u32 {
        2
    }
}

/// One region, one handler.
mod single {
    use super::*;

    #[derive(Debug)]
    pub struct Solo;
    impl State for Solo {
        type Parent = NoParent;
        fn define(def: &mut Definition<Self>) {
            def.region::<Solo_Leaf>();
        }
    }

    #[derive(Debug, Default)]
    pub struct Solo_Leaf;
    impl State for Solo_Leaf {
        type Parent = Solo;
        fn define(def: &mut Definition<Self>) {
            def.trigger::<Get>();
        }
    }
    impl Trigger<Get> for Solo_Leaf {
        fn handle(&mut self, _: &Get, _: &mut Context<Self>) -> u32 {
            7
        }
    }
}

#[test]
fn update_trigger_is_active_and_state_do_not_allocate() {
    let mut solo = Statechart::new(single::Solo);
    let mut chart = Statechart::new(Root);
    // Warm up.
    solo.update();
    chart.update();
    assert_eq!(solo.trigger(Get), Ok(7));
    assert_eq!(chart.trigger_all(Get), Ok(vec![1, 2]));

    let n = allocations_in(|| {
        for _ in 0..100 {
            solo.update();
            chart.update();
        }
    });
    assert_eq!(n, 0, "update");

    let n = allocations_in(|| {
        for _ in 0..100 {
            assert_eq!(solo.trigger(Get), Ok(7));
            // Both regions handle it; `trigger` keeps the first output.
            assert_eq!(chart.trigger(Get), Ok(1));
        }
    });
    assert_eq!(n, 0, "trigger");

    let n = allocations_in(|| {
        for _ in 0..100 {
            assert!(chart.is_active::<Root_B_X>());
            assert!(chart.state::<Root_B_X>().is_some());
        }
    });
    assert_eq!(n, 0, "is_active / state");
}
