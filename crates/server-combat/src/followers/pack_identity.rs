//! Shared source formation identity; never proximity-based clustering.
pub fn pack_guid(spawn_guid: u32, formation_leader: Option<u32>) -> u32 {
    formation_leader.unwrap_or(spawn_guid)
}
