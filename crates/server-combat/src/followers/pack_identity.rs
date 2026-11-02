//! Shared source formation identity; never proximity-based clustering.
pub fn pack_guid(spawn_guid: u64, formation_leader: Option<u64>) -> u64 {
    formation_leader.unwrap_or(spawn_guid)
}
