//! Helpers shared by the integration tests: a per-thread event log (each test runs on its own
//! thread, so tests do not see each other's entries).
#![allow(dead_code, unused_macros)]

use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Appends one entry, e.g. `log("enter", short_name::<Self>())`.
pub fn log(what: &str, who: &str) {
    LOG.with(|l| l.borrow_mut().push(format!("{what} {who}")));
}

/// Returns and clears the log.
pub fn take_log() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// The log without `update ...` entries.
pub fn take_log_without_updates() -> Vec<String> {
    take_log()
        .into_iter()
        .filter(|entry| !entry.starts_with("update "))
        .collect()
}

/// `Root_A_X` for `some::module::Root_A_X`.
pub fn short_name<S>() -> &'static str {
    std::any::type_name::<S>().rsplit("::").next().unwrap()
}

/// Implements `on_enter`, `on_exit` and `update` so that each one only logs. Needs
/// `#[macro_use] mod common;`.
macro_rules! logging_hooks {
    () => {
        fn on_enter(&mut self, _: &mut Context<Self>) {
            common::log("enter", common::short_name::<Self>());
        }
        fn on_exit(&mut self, _: &mut ExitContext<Self>) {
            common::log("exit", common::short_name::<Self>());
        }
        fn update(&mut self, _: &mut Context<Self>) {
            common::log("update", common::short_name::<Self>());
        }
    };
}
