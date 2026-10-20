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
cargo run --example timer                    # runs forever, printing the active configuration each second; Ctrl-C to stop
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cargo build && python3 scripts/check_compile_fail.py .   # optional: each compile_fail doctest fails with its listed error code
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, `cargo test --all-targets` and `cargo test --doc`, all with `RUSTFLAGS=-D warnings`.

Test files in `tests/` (shared helpers, such as the per-thread event log, are in `tests/common/mod.rs`):

| File | Covers |
|---|---|
| `integration_test.rs` | the original suite ported to the new API (`test_1`/`test_2`/`test_3` modules), incl. the duplicate-state `assert!` |
| `user_example.rs` | the A / B1 / B2 / `Foo` example from the original request |
| `regions.rs` | parallel regions: independence, per-region dispatch, cross-region transitions |
| `triggers.rs` | dispatch on one chain: deepest handler wins, fallback to ancestors, unregistered handlers |
| `transitions.rs` | LCA transitions: siblings, into inactive ancestors, self, to an ancestor, to the root, one region only |
| `batch.rs` | request order, dropping requests of exited states, `on_enter` requests |
| `lifecycle.rs` | `on_enter` / `on_exit` / `update` ordering and constructor timing |
| `ancestors.rs` | `parent()` / `parent_mut()` / `ancestor()` / `ancestor_mut()` from every hook |
| `introspection.rs` | Debug format, `get_recursive_active_type_ids`, `is_active`, `state` |
| `scenarios.rs` | end-to-end edge cases on a chart with nested parallel regions |
| `validation.rs` | runtime checks (double trigger, `child_explicit` composite, `on_enter` cycle guard, unlisted transition target) and panic robustness |
| `module_paths.rs` | the API through module paths only, and the prelude naming the same items |
| `timer_test.rs` | the timer example's states, loaded with `#[path]` |

## Architecture

Users define states as plain structs implementing `state::State`; a struct's fields are the state's data. `statechart::Statechart<R>` owns the active configuration of the chart rooted at `R`. The code has no `unsafe` (`#![forbid(unsafe_code)]`).

- **`State` trait** (`src/state.rs`): `State: Any + Debug + Sized` with `type Parent` (the root uses the uninhabited `NoParent`) and four defaulted methods: `define(def)`, `on_enter(&mut self, ctx: &mut Context<Self>)`, `on_exit(&mut self, ctx: &mut ExitContext<Self>)` and `update(&mut self, ctx: &mut Context<Self>)`. The hook is called `update` (not `tick`). `type Parent` is the static hierarchy: it types `ctx.parent()`, lets a transition build a target's inactive ancestors, and powers `InChart<R>` / `RootOf<S>`, which turn "is this state part of my chart?" into a compile error.
- **`define()`** (`src/definition.rs`): a static function (no instance) that declares regions and triggers on a `Definition<Self>`. `def.region::<Initial>()` (or `region_with(|parent| ..)`) starts a region by naming its initial child; `.child::<C>()`, `.child_with(ctor)` and `.child_explicit::<C>()` add the region's other children. A leaf has no regions, a plain composite has one, two or more run in parallel. `child::<C>()` only compiles if `C::Parent == Self`. `def.trigger::<E>()` registers a handled trigger.
- **Engine** (`src/engine.rs`, private): `Registry::build::<R>()` runs every `define` once per `Statechart::new` and stores, per state type, its region index in the parent, each region's initial child, the trigger table, the constructor and the cached path from the root. The active configuration is a tree of `Node { state, idx, id, regions }`, one active child per region; inactive states are never built. Ancestor access is safe: each traversal splits a node into its state and its regions and passes the ancestors down as a linked list of stack frames (`Frame` / `Nil`).
- **Construction checks** (always on, debug and release): a state type appearing twice (`assert!`, message ``State type `X` appears more than once in the statechart hierarchy``; tests depend on it), a trigger registered twice, and a composite registered with `child_explicit`. A transition to a state whose parent does not list it panics at request time, in the requesting hook (`#[track_caller]`).
- **Contexts** (`src/context.rs`): `Context` (for `update`, `on_enter`, trigger handlers) and `ExitContext` (for `on_exit`) both offer `parent()` / `parent_mut()` (typed, not an `Option`, absent on the root), `ancestor::<A>()` / `ancestor_mut::<A>()` (any depth, `None` for non-ancestors, including other regions) and `is_active::<T>()`. Only `Context` has `transition(target)`; `on_exit` cannot request transitions.
- **Triggers** (`src/trigger.rs`): a trigger type implements `Event { type Output }`; a state handles it with `impl Trigger<E> for S { fn handle(&mut self, &E, ctx) -> E::Output }` **and** `def.trigger::<E>()` in `define`. A forgotten registration is not detected: the handler never runs and the result is `Err(NotHandled)` unless an ancestor handles it. Dispatch is post-order: on each active branch the deepest registered handler runs, and a shared ancestor runs at most once, only for branches nobody below handled. `chart.trigger(e)` returns the first output in execution order, `chart.trigger_all(e)` all of them; both return `Err(NotHandled)` when no handler ran.
- **Transitions** are LCA-based and any-to-any. `ctx.transition(Target { .. })` queues a request; applying it finds the deepest active state on the target's path (the LCA), exits that state's region leading to the target (deepest first), then enters down to the target, building inactive ancestors with their registered constructors and giving every other newly entered region its initial child. Only that region changes: a transition into one region of a parallel state leaves the sibling regions untouched, while targeting the parallel state itself resets all its regions. Targeting an active state is an external self-transition; targeting the root restarts the chart.
- **Batches.** Nothing changes mid-traversal. The requests of one `update` / trigger dispatch / `new` are applied afterwards in request order (pre-order for `update`, handler order for triggers); requests made by `on_enter` during the batch are appended; a request is dropped if its requester instance was exited earlier in the batch (each activation has a fresh id). More than 10 000 transitions in one batch panic (`on_enter` cycle guard).
- **Lifecycle ordering.** `Statechart::new` enters the root and then all initial descendants. `on_enter` runs parent before child (initial children included), regions in declaration order; a child's constructor runs right after its parent's `on_enter`. `on_exit` runs deepest first. `update` runs parent before children, regions in declaration order.
- **Introspection.** `Debug` / `get_debug_name()` print the active configuration with each state's own `Debug`: `"A -> A_B"` for one chain, `"A -> [A_B -> A_B_X, A_C]"` for parallel regions; tests assert on these strings. `get_recursive_active_type_ids()` is post-order without the root (deepest first, regions in order; `[A_B_C, A_B]` for a chain). `is_active::<T>()`, `state::<T>()`, `root()` and `root_mut()` query the chart.

## Conventions

- State struct names are breadcrumbs of their ancestors: `Parent_Child_Grandchild`, e.g. `Timer_Active_Running` is `Running`, a child of `Active`, a child of the root `Timer`. Sibling names are never joined (`Timer_Active_Running` and `Timer_Active_Paused` are siblings; `Timer_Running_Paused` would be wrong). Hence `#![allow(non_camel_case_types)]` in `lib.rs`, tests and the example.
- User code imports `use statechart::prelude::*;` (`State`, `NoParent`, `Definition`, `Context`, `ExitContext`, `Event`, `Trigger`, `NotHandled`, `Statechart`). Every item also stays reachable through its module path (`statechart::state::State`, `statechart::statechart::Statechart`, ...); the crate root re-exports nothing but the modules, and `tests/module_paths.rs` checks both.
- One test file per area under `tests/` (see the table above); public items get rustdoc (`#![warn(missing_docs)]`), and compile-time checks get a `compile_fail,EXXXX` doctest in `src/`.
- `tests/timer_test.rs` pulls the example's state modules in with `#[path = "../examples/timer/..."]`, so changing the public API breaks that test until the example is updated too.

## Agent skills

### Issue tracker

Issues are tracked in this repo's GitHub Issues. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the default five triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context, with everything under `docs/`: the glossary is `docs/GLOSSARY.md` (not the repo root) and ADRs go in `docs/adr/`. See `docs/agents/domain.md`.
