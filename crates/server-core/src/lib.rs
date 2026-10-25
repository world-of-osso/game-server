//! Shared server records and immutable catalogs, below gameplay execution.
pub mod catalog_data;
pub mod session_state;
pub mod spell_info;
#[cfg(any(test, feature = "test-support"))]
pub mod test_data;

#[cfg(test)]
#[path = "test_data/catalog_cache_tests.rs"]
mod native_catalog_cache_tests;
#[cfg(test)]
#[path = "test_data/catalog_data_tests.rs"]
mod native_catalog_data_tests;
