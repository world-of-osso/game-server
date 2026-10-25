//! Map identity, metadata, admission and lock rules; no server gameplay dependency.
pub mod maps;
pub mod terrain_height;
#[cfg(any(test, feature = "test-support"))]
pub mod test_data;
pub mod world_area_triggers;
pub mod world_data;
