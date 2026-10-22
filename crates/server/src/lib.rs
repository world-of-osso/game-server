//! Standalone authoritative gameplay libraries; no server runtime initialization.
pub mod character_data;
pub mod item_instance;
pub mod mythic_plus_catalog;
pub mod mythic_plus_history;
pub mod mythic_plus_runtime;
pub mod pet_battle;
pub mod raid_history_store;

// Keep fixture cache contracts testable without compiling the server entry point.
#[cfg(test)]
#[path = "test_data/catalog_cache_tests.rs"]
mod native_catalog_cache_tests;
