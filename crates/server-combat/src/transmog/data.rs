//! Embedded local-CASC Retail12.1.0.69933 metadata. Regenerate through
//! scripts/import_transmog_catalogue.py; world.db is never modified.
use serde::Deserialize;
use shared::protocol::*;

#[derive(Deserialize)]
pub(super) struct AuthoredSet {
    pub set_id: u32,
    pub parent_set_id: u32,
    pub name: String,
    pub source_ids: Vec<u32>,
    pub class_mask: u32,
    pub condition_id: u32,
}

impl AuthoredSet {
    /// Class-usable and without an unevaluated PlayerCondition.
    pub fn usable_by(&self, class: u8) -> bool {
        let class_mask = 1u32 << (class - 1);
        (self.class_mask == 0 || self.class_mask & class_mask != 0) && self.condition_id == 0
    }
}

#[derive(Deserialize)]
struct AuthoredSlot {
    id: u32,
    inventory_slot_id: u8,
    flags: u32,
    collection_type: u8,
}

#[derive(Deserialize, Default)]
pub(super) struct Wardrobe {
    sets: Vec<AuthoredSet>,
    pub outfit_entries: Vec<TransmogOutfitEntry>,
    illusions: Vec<TransmogIllusion>,
    slots: Vec<AuthoredSlot>,
    outfit_options: Vec<TransmogOutfitOption>,
}

impl Wardrobe {
    pub fn load_embedded() -> Self {
        serde_json::from_str(include_str!(
            "../../../../scripts/reference/retail-12.1.0.69933/transmog.json"
        ))
        .expect("decode pinned Retail transmog catalogue")
    }

    pub fn sets(&self) -> &[AuthoredSet] {
        &self.sets
    }

    pub fn populate(&self, state: &mut TransmogState) {
        state.outfit_entries = self.outfit_entries.clone();
        state.illusions = self.illusions.clone();
        // TrinityCore a352b1fa CollectionMgr::LoadAccountTransmogIllusions
        // grants these seven static DB2 identities to every account.
        const DEFAULT_ILLUSIONS: [u32; 7] = [3, 13, 22, 23, 34, 43, 44];
        for illusion in &mut state.illusions {
            illusion.collected = DEFAULT_ILLUSIONS.contains(&illusion.id);
        }
        state.outfit_slots = self
            .slots
            .iter()
            .map(|entry| TransmogOutfitSlot {
                id: entry.id,
                slot: EquipmentSlot::ALL
                    .into_iter()
                    .find(|slot| slot.inv_slot_id() == entry.inventory_slot_id)
                    .expect("authored transmog inventory slot"),
                flags: entry.flags,
                collection_type: entry.collection_type,
            })
            .collect();
        state.outfit_options = self.outfit_options.clone();
    }
}
