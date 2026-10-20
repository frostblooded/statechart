# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`statechart` is a Rust (edition 2024) library for hierarchical state machines (statecharts). It has no dependencies. A runnable countdown-timer example lives in `examples/timer/`.

## Working rules

These mirror the project instructions; keep the two in sync whenever either changes.

- Before making any plans or changes, grill the user with clarifying questions until you are exactly sure what they want.
- Keep the project instructions in sync with this CLAUDE.md.
- Follow TDD: write failing tests first, then make the changes that pass them.
- When asking the user questions, always use the multiple choice question widget.
- When a PR is done (work finished and tests pass), merge it into `master` automatically without waiting for the user.
- Updates to CLAUDE.md that only sync it with the project instructions need no PR; commit them directly to `master`.

## Commands

```sh
cargo build
cargo test                                   # all tests: tests/*.rs (one file per area, see below) plus doctests
cargo test test_basic_transition             # single test by name
cargo test test_3::                          # all tests in one module of tests/integration_test.rs
cargo test --test timer_test                 # one test file
cargo test --doc                             # doctests only (crate example, rustdoc examples, compile_fail checks)
cargo test --release --all-targets           # the always-on checks must also fire without debug assertions (D1)
cargo run --example timer                    # runs forever, printing the active configuration each second; Ctrl-C to stop
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo build && python3 scripts/check_compile_fail.py .   # optional: each compile_fail doctest fails with its listed error code
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, `cargo test --all-targets`, `cargo test --doc` and `cargo test --release --all-targets`, all with `RUSTFLAGS=-D warnings`. The release run is what tells `assert!` from `debug_assert!`, so it is the only guard for user decision D1 (the duplicate-state check fires in every build).

Test files in `tests/` (shared helpers, such as the per-thread event log, are in `tests/common/mod.rs`):

| File | Covers |
|---|---|
| `integration_test.rs` | the original suite ported to the new API (`test_1`/`test_2`/`test_3` modules), incl. the duplicate-state `assert!` |
| `user_example.rs` | the A / B1 / B2 / `Foo` example from the original request |
| `regions.rs` | parallel regions: independence, per-region dispatch, cross-region transitions |
| `triggers.rs` | dispatch on one chain: deepest handler wins, fallback to ancestors, unregistered handlers |
| `transitions.rs` | LCA transitions: siblings, into inactive ancestors, self, to an ancestor, to the root (the new root is live and the old descendants exit under the old root), one region only |
| `batch.rs` | request order, dropping requests of exited states, `on_enter` requests (which wait until later regions are entered) |
| `lifecycle.rs` | `on_enter` / `on_exit` / `update` ordering, constructor timing (also in later regions and during transitions), when states are dropped (post-order, also across regions) |
| `ancestors.rs` | `parent()` / `parent_mut()` / `ancestor()` / `ancestor_mut()` from every hook; `ancestor*` are unbounded (generic helpers, `None` for another chart's state) |
| `introspection.rs` | Debug format, `get_recursive_active_type_ids` (post-order, not sorted by depth), `is_active`, `state`, `state_mut` |
| `scenarios.rs` | end-to-end edge cases on a chart with nested parallel regions |
| `validation.rs` | runtime checks (double trigger, `child_explicit` composite, `on_enter` cycle guard with its exact per-batch bound, which counts applied transitions and not dropped requests, unlisted transition target or ancestor) and panic robustness, including panics in `on_enter` / `on_exit` / constructors while a transition is applied, a restart whose old root's `on_exit` panics (and a panic in the batch that later enters that root), the rest of an interrupted batch never being applied, and the targets of discarded requests being dropped while the panic unwinds |
| `panic_locations.rs` | panics about user mistakes point at the user's line (`#[track_caller]`); one test, because it swaps the panic hook |
| `allocations.rs` | `update`, `trigger`, `is_active` and `state` do not allocate, nor do (once warmed up) `update` and `trigger` that apply transitions between zero-sized leaves (a counting global allocator, the only `unsafe` in the repo) |
| `module_paths.rs` | the API through module paths only, and the prelude naming the same items |
| `timer_test.rs` | the timer example's states, loaded with `#[path]`: incl. that Running answers `GetRemaining` itself (live value, not `Timer_Active`'s fallback) and that `Pause` ticks once more |

## Architecture

Users define states as plain structs implementing `state::State`; a struct's fields are the state's data. `statechart::Statechart<R>` owns the active configuration of the chart rooted at `R`. The library has no `unsafe` (`#![forbid(unsafe_code)]`); the only `unsafe` is the counting allocator in `tests/allocations.rs`.

- **`State` trait** (`src/state.rs`): `State: Any + Debug + Sized` with `type Parent` (the root uses the uninhabited `NoParent`) and four defaulted methods: `define(def)`, `on_enter(&mut self, ctx: &mut Context<Self>)`, `on_exit(&mut self, ctx: &mut ExitContext<Self>)` and `update(&mut self, ctx: &mut Context<Self>)`. The hook is called `update` (not `tick`). `type Parent` is the static hierarchy: it types `ctx.parent()`, lets a transition build a target's inactive ancestors, and powers `InChart<R>` / `RootOf<S>`, which turn "is this state part of my chart?" into a compile error. A non-root state of another chart gets `SameRoot`'s message; everything else gets `InChart`'s own message, label and note: the root of another chart (e.g. `chart.is_active::<OtherRoot>()`, no box involved), a boxed state or a reference (`ctx.transition(Box::new(Target))`), and types that are not states at all. So `InChart`'s note must stay true for all of those (its boxing advice is conditional). `SameRoot`'s note also tells generic code to bound with `T: InChart<RootOf<S>>` (in a hook of `S`) or `T: InChart<R>` (for `Statechart<R>`), since rustc's own suggestion names the private `sealed::SameRoot`; `InChart`'s rustdoc has a "Generic code" doctest.
- **`define()`** (`src/definition.rs`): a static function (no instance) that declares regions and triggers on a `Definition<Self>`. `def.region::<Initial>()` (or `region_with(|parent| ..)`) starts a region by naming its initial child; `.child::<C>()`, `.child_with(ctor)` and `.child_explicit::<C>()` add the region's other children. A leaf has no regions, a plain composite has one, two or more run in parallel. `child::<C>()` only compiles if `C::Parent == Self`. `def.trigger::<E>()` registers a handled trigger.
- **Engine** (`src/engine.rs`, private): `Registry::build::<R>()` runs every `define` once per `Statechart::new` and stores, per state type, its name, its region index in the parent, each region's initial child, the trigger table, the constructor and the cached path from the root. The active configuration is a tree of `Node { state, idx, id, regions }`, one active child per region; inactive states are never built. Ancestor access is safe: each traversal splits a node into its state and its regions and passes the ancestors down as a linked list of stack frames (`Frame` / `Nil`).
- **Panic safety of transitions.** A state is placed in the tree (and becomes live) just before its `on_enter`; a subtree is detached from the tree (its region holds a vacant `Node`, `idx == VACANT`) before it is exited, and each exited state is dropped right after its `on_exit`. So if a hook or constructor panics mid-transition, the tree only holds entered, not-exited states (with one exception, the pending root of an interrupted restart, below); `ResyncOnUnwind` (a drop guard around `run_batch`) then rebuilds the live set from the tree, without running user code. A region left vacant prints as `(empty)`, is skipped by `update`, bubbles triggers straight to its parent, and is filled by the next transition into it. A root restart swaps the new root in before the old root's own `on_exit`, so the root slot is never vacant; the instance check in `Runtime::deactivate` keeps the old root from removing the new root's live entry. The new root is in the tree but not entered while the old root's `on_exit` runs (`Runtime::root_pending_enter`); if that `on_exit` panics, the flag stays set and `Registry::enter_pending_root`, called first by `update` / `trigger` / `trigger_all`, enters the new root (and applies its `on_enter` requests) before anything else runs on it. So `update`, handlers and `on_exit` never run on a state whose `on_enter` has not run, and the tree holds only entered, not-exited states apart from that pending root. `Node`'s `Drop` drops children before the state (post-order). Documented in `Statechart`'s rustdoc ("When states are dropped", "Panics in hooks"); tested in `validation.rs` and `lifecycle.rs`.
- **Construction checks** (always on, debug and release): a state type appearing twice (`assert!` in `Region::push`, `#[track_caller]`, so it points at the user's second `region*` / `child*` call; message ``State type `X` appears more than once in the statechart hierarchy: `P` lists it twice in `define` ``; tests depend on the prefix; `engine::register` keeps the bare message as a backstop), a trigger registered twice (`#[track_caller]`), and a composite registered with `child_explicit` (the message names the line that registered it). A transition to a state that is not reachable through `define` panics at request time, in the requesting hook (`#[track_caller]`): ``its parent `P` does not list it in `define` `` when the target's own parent forgot it, ``its ancestor `A` is not listed by its parent `Q` in `define` `` when a state higher up was forgotten (found by `sealed::Sealed::unlisted_link`, a method of `ParentSlot`'s crate-private supertrait; it takes a `sealed::Registered` that only the crate can build, so code outside the crate cannot run it, and `ParentSlot` itself has only the hidden `RootOf` GAT). From `on_enter` during a transition, that panic interrupts the transition like any hook panic. The `# Panics` sections of `new`, `update`, `trigger` and `trigger_all` keep the two timings apart: a request from `update` or a handler panics during the traversal, before anything is applied (with `trigger_all` the outputs collected so far are lost); a request from `on_enter` (including while `new` enters the chart) and the cycle guard panic while the transitions are applied.
- **Contexts** (`src/context.rs`): `Context` (for `update`, `on_enter`, trigger handlers) and `ExitContext` (for `on_exit`) both offer `parent()` / `parent_mut()` (typed, not an `Option`, absent on the root; their bound is the crate-private `sealed::ParentState`, so generic code over `S: State` writes `where S::Parent: State`, as their rustdoc shows), `ancestor::<A>()` / `ancestor_mut::<A>()` (any depth, `None` for non-ancestors, including other regions and other charts; deliberately bound only by `State`, not `InChart`, so generic helpers can call them) and `is_active::<T>()`. Only `Context` has `transition(target)`; `on_exit` cannot request transitions. Every accessor borrows the context, so a hook can hold only one `&mut` ancestor at a time and not together with another accessor or `is_active` (E0502); `Context`'s rustdoc ("One mutable borrow at a time") shows reading into locals first, plus a `compile_fail,E0502` doctest. The pure queries (these accessors and `Statechart::is_active` / `state` / `state_mut` / `root` / `root_mut` / `get_debug_name` / `get_recursive_active_type_ids`) are `#[must_use]`.
- **Triggers** (`src/trigger.rs`): a trigger type implements `Event { type Output }`; a state handles it with `impl Trigger<E> for S { fn handle(&mut self, &E, ctx) -> E::Output }` **and** `def.trigger::<E>()` in `define`. A forgotten registration is not detected: the handler never runs and the result is `Err(NotHandled)` unless an ancestor handles it. Dispatch is post-order: on each active branch the deepest registered handler runs, and a shared ancestor runs at most once, only for branches nobody below handled. `chart.trigger(e)` returns the first output in execution order, `chart.trigger_all(e)` all of them; both return `Err(NotHandled)` when no handler ran. Every eligible handler runs either way; the outputs go into an `engine::Sink` (`First` keeps the first without allocating, `All` collects).
- **Transitions** are LCA-based and any-to-any. `ctx.transition(Target { .. })` queues a request; applying it finds the LCA, the deepest active state on the path from the root to the target *excluding the target* (always a strict ancestor), exits the LCA's region leading to the target (deepest first), then enters down to the target, building inactive ancestors with their registered constructors and giving every other newly entered region its initial child. Only that region changes: a transition into one region of a parallel state leaves the sibling regions untouched, while targeting the parallel state itself (its parent is then the LCA) resets all its regions. Targeting an active state is an external self-transition; targeting the root restarts the chart.
- **Batches.** Nothing changes mid-traversal. The requests of one `update` / trigger dispatch / `new` are applied afterwards in request order (pre-order for `update`, handler order for triggers); requests made by `on_enter` during the batch are appended; a request is dropped if its requester instance was exited earlier in the batch (each activation has a fresh id). More than 10 000 applied transitions in one batch panic (`on_enter` cycle guard, counted per batch, not per chart, dropped requests not counted; the message names the last request). The batch queue (`Runtime::queue`) and the request buffer keep their capacity between batches, so transitions between zero-sized leaves do not allocate. Requests discarded by a panic are dropped while it unwinds, so their targets release resources at once: `Registry::update_root` / `dispatch_root` wrap the traversal in `DiscardRequestsOnUnwind`, `ResyncOnUnwind` also clears `Runtime::requests`, and the batch's taken queue is a local. Every operation still clears leftover requests first.
- **Lifecycle ordering.** `Statechart::new` enters the root and then all initial descendants. `on_enter` runs parent before child (initial children included), regions in declaration order, each region's whole subtree before the next region. A child's constructor runs when its region is entered: after its parent's `on_enter` and after the parent's earlier regions (whose `on_enter` may have changed the parent); during a transition into an already active parent, when the transition is applied. It sees the parent's data as it is then. `on_exit` runs in post-order (deepest first within a branch, regions in declaration order, so a shallow state in an earlier region exits before a deeper one in a later region), and each state is dropped right after its `on_exit`, before anything is entered. Dropping the chart drops the states in the same post-order without `on_exit`. `update` runs parent before children, regions in declaration order.
- **Introspection.** `Debug` / `get_debug_name()` print the active configuration with each state's own `Debug`: `"A -> A_B"` for one chain, `"A -> [A_B -> A_B_X, A_C]"` for parallel regions; tests assert on these strings. `get_recursive_active_type_ids()` is post-order without the root (deepest first within a branch, regions in order; `[A_B_C, A_B]` for a chain, `[R_A, R_B_1, R_B]` for `R -> [R_A, R_B -> R_B_1]`, so not sorted by depth). `is_active::<T>()`, `state::<T>()`, `state_mut::<T>()`, `root()` and `root_mut()` query the chart.
- **Limitations** (documented on `Statechart`): not `Send` / `Sync`, not `UnwindSafe` / `RefUnwindSafe` (catch with `catch_unwind(AssertUnwindSafe(..))`, shown in a doctest under "Panics in hooks"), no internal events (hooks cannot fire triggers), one `&mut` ancestor at a time in a hook, states are `'static`, and so are triggers and their outputs (they are passed type-erased, so a trigger cannot hold non-`'static` borrows; a `&'static str` is fine; per-call data goes in owned trigger fields or in the root's data via `root_mut()` before `update()`).

## Conventions

- State struct names are breadcrumbs of their ancestors: `Parent_Child_Grandchild`, e.g. `Timer_Active_Running` is `Running`, a child of `Active`, a child of the root `Timer`. Sibling names are never joined (`Timer_Active_Running` and `Timer_Active_Paused` are siblings; `Timer_Running_Paused` would be wrong). Hence `#![allow(non_camel_case_types)]` in `lib.rs`, tests and the example.
- User code imports `use statechart::prelude::*;` (`State`, `NoParent`, `Definition`, `Context`, `ExitContext`, `Event`, `Trigger`, `NotHandled`, `Statechart`). Every item also stays reachable through its module path (`statechart::state::State`, `statechart::statechart::Statechart`, ...); the crate root re-exports nothing but the modules, and `tests/module_paths.rs` checks both.
- One test file per area under `tests/` (see the table above); public items get rustdoc (`#![warn(missing_docs)]`), and compile-time checks get a `compile_fail,EXXXX` doctest in `src/`. Lints have no error code: the `#[must_use]` checks are plain `compile_fail` doctests with `# #![deny(unused_must_use)]` (rustdoc allows `unused` by default), which `scripts/check_compile_fail.py` skips.
- `tests/timer_test.rs` pulls the example's state modules in with `#[path = "../examples/timer/..."]`, so changing the public API breaks that test until the example is updated too.

## Agent skills

### Issue tracker

Issues are tracked in this repo's GitHub Issues. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the default five triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context, with everything under `docs/`: the glossary is `docs/GLOSSARY.md` (not the repo root) and ADRs go in `docs/adr/`. See `docs/agents/domain.md`.
