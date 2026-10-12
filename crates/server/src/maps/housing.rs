//! Property-owned copies, with admission issued only by the housing authority.
//! Housing Map.db2 types have no MapDifficulty rows or dungeon lock semantics.
use super::super::InMap;
use super::super::store::{InstanceType, MapStore};
use super::{InstanceManager, ResolvedMap};
use shared::protocol::TransferAbortReason;

impl InstanceManager {
    /// Called after the service checks neighborhood membership or house access.
    pub(crate) fn authorize_housing_entry(
        &mut self,
        store: &MapStore,
        map_id: u32,
        property_id: u64,
        character_id: u64,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        let housing_map = store.map(map_id).is_some_and(|entry| {
            matches!(
                entry.instance_type,
                InstanceType::House | InstanceType::Neighborhood
            )
        });
        if !housing_map {
            return Err(TransferAbortReason::MapNotAllowed);
        }
        let key = (map_id, property_id);
        let (map, created) = match self.housing_copies.get(&key) {
            Some(&map) => (map, false),
            None => {
                let map = InMap {
                    map_id,
                    instance_id: self.generate_instance_id(),
                };
                self.housing_copies.insert(key, map);
                self.housing_content_pending.insert(map);
                (map, true)
            }
        };
        self.housing_by_character.insert(character_id, map);
        Ok(ResolvedMap { map, created })
    }

    pub(crate) fn authorized_housing_map(&self, character_id: u64, map_id: u32) -> Option<InMap> {
        self.housing_by_character
            .get(&character_id)
            .copied()
            .filter(|map| map.map_id == map_id)
    }

    /// Reservation allocates identity; only the first arrival initializes content.
    pub(super) fn create_authorized_housing_map(
        &mut self,
        character_id: u64,
        map_id: u32,
    ) -> Result<ResolvedMap, TransferAbortReason> {
        let map = self
            .authorized_housing_map(character_id, map_id)
            .ok_or(TransferAbortReason::MapNotAllowed)?;
        let created = self.housing_content_pending.remove(&map);
        Ok(ResolvedMap { map, created })
    }

    pub(crate) fn revoke_housing_entry(&mut self, character_id: u64) {
        self.housing_by_character.remove(&character_id);
    }

    pub(crate) fn housing_property(&self, map: InMap) -> Option<u64> {
        self.housing_copies
            .iter()
            .find_map(|(&(map_id, property_id), &copy)| {
                (copy == map && map_id == map.map_id).then_some(property_id)
            })
    }
}
