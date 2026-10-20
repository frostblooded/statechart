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
cargo test                                   # all tests (tests/integration_test.rs, tests/timer_test.rs)
cargo test test_basic_transition             # single test by name
cargo test test_3::                          # all tests in one module of the integration test file
cargo test --test timer_test                 # one test file
cargo run --example timer                    # runs forever, printing the active chain each second; Ctrl-C to stop
cargo clippy
```

## Architecture

Users define states as plain structs implementing `CustomStateTrait` (`src/custom_state_trait.rs`). The library wraps each one in an internal `State` node (`src/state.rs`), and `Statechart` (`src/statechart.rs`) owns the root node.

- **Hierarchy is a single active chain, not a tree of all states.** Each `State` holds exactly one optional active `child`. Inactive sibling states are never constructed; a transition builds the new state's struct and replaces the child.
- **Children are declared by `TypeId`.** A state lists its allowed children via `get_possible_children_type_ids()` and supplies its default via `get_initial_child()`. `State::new` recursively builds the initial-child chain and `debug_assert!`s the initial child is in the allowed list (there is a `should_panic` test that depends on the exact message).
- **`CustomStateTrait: Any + Debug`.** The `Any` supertrait is what makes `.type_id()` on a `Box<dyn CustomStateTrait>` return the concrete type, which transition matching relies on. `Debug` drives `Statechart::get_debug_name()`, which formats the active chain as `"A -> A_A"`; tests assert on that string.
- **Update cycle.** `Statechart::update()` creates a `StatechartUpdateContext`, calls `update` top-down through the active chain, then applies the collected transitions. States request a transition with `context.transition(Box::new(NewState::...))`; nothing changes mid-update.
- **Transition semantics (current limitations).** `apply_transitions` runs only on the root and only matches against the root's direct possible children; transitions targeting deeper levels are silently ignored (see the `TODO` about cross-level transitions). On a match it calls `on_exit` on the old child and `on_enter` on the new one; `Statechart::new` calls `on_enter` on the root only, and initial children built by `State::new` do not get `on_enter`. Triggers/events are not implemented yet (`TODO` in `statechart.rs`).
- `get_recursive_active_type_ids()` returns the active descendants' `TypeId`s deepest-first, excluding the root.

## Conventions

- State struct names use `Parent_Child` style (e.g. `A_A`, `Timer_Running`), hence `#![allow(non_camel_case_types)]` in `lib.rs` and tests.
- Public API is reached through module paths (`statechart::statechart::Statechart`, `statechart::custom_state_trait::CustomStateTrait`); there are no re-exports at the crate root.
- `tests/timer_test.rs` pulls the example's state modules in with `#[path = "../examples/timer/..."]`, so changing the public API breaks that test until the example is updated too.

## Agent skills

### Issue tracker

Issues are tracked in this repo's GitHub Issues. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the default five triage labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context, with everything under `docs/`: the glossary is `docs/GLOSSARY.md` (not the repo root) and ADRs go in `docs/adr/`. See `docs/agents/domain.md`.
