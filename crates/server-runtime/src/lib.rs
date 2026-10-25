//! Transport-free ECS state shared by gameplay and native adapters.

pub mod messaging;
pub mod replication_dirty;

pub use replication_dirty::ReplicationDirtyQueue;
