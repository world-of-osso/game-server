//! Embedded local-CASC Retail12.1.0.69933 metadata. Regenerate through
//! scripts/import_transmog_catalogue.py; world.db is never modified.
use serde::Deserialize;
use shared::protocol::*;
use std::collections::HashSet;

#[derive(Deserialize)]
pub(super) struct AuthoredSet {
    #[serde(flatten)]
    pub view: TransmogSet,
    pub class_mask: u32,
    pub condition_id: u32,
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

    pub fn populate(&self, state: &mut TransmogState, class: u8) {
        let ids: HashSet<_> = state
            .catalogue
            .iter()
            .map(|entry| entry.source.source_id)
            .collect();
        let class_mask = 1u32 << (class - 1);
        state.sets = self
            .sets
            .iter()
            .filter(|entry| {
                let matches_class = entry.class_mask == 0 || entry.class_mask & class_mask != 0;
                let readable_condition = entry.condition_id == 0;
                matches_class
                    && readable_condition
                    && entry.view.source_ids.iter().any(|id| ids.contains(id))
            })
            .map(|entry| entry.view.clone())
            .collect();
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
