//! Retail DyeCostIsValid spends one item per new/changed nondefault slot.
//! HousingDecorDyeSlot generated docs explicitly accept colors of any category.
use super::{number, read_build_key, read_rows};
use rusqlite::Connection;
use shared::protocol::DecorDye;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub(crate) struct DyeCatalog {
    slots: BTreeMap<u32, u32>,
    colors: BTreeMap<u32, u32>,
}

impl DyeCatalog {
    pub(crate) fn read(db: &Connection) -> Result<Self, String> {
        read_build_key(db)?;
        Ok(Self {
            slots: read_rows(db, "DecorDyeSlot")?
                .into_iter()
                .map(|(id, fields)| Ok((id, number(&fields, "HouseDecorID")?)))
                .collect::<Result<_, String>>()?,
            colors: read_rows(db, "DyeColor")?
                .into_iter()
                .map(|(id, fields)| Ok((id, number(&fields, "ItemID")?)))
                .collect::<Result<_, String>>()?,
        })
    }

    pub(crate) fn costs(
        &self,
        decor_id: u32,
        current: &[DecorDye],
        changes: &[DecorDye],
    ) -> Result<BTreeMap<u32, u32>, String> {
        let mut seen = BTreeSet::new();
        let mut costs = BTreeMap::new();
        for change in changes {
            if !seen.insert(change.slot_id) {
                return Err("A dye preview repeats a slot.".into());
            }
            if self.slots.get(&change.slot_id) != Some(&decor_id) {
                return Err("This dye slot does not belong to the selected decor.".into());
            }
            let item = change
                .color_id
                .map(|color| {
                    self.colors
                        .get(&color)
                        .copied()
                        .filter(|item| *item != 0)
                        .ok_or("This dye color has no authenticated dye item.".to_string())
                })
                .transpose()?;
            let before = current
                .iter()
                .find(|saved| saved.slot_id == change.slot_id)
                .and_then(|saved| saved.color_id);
            if before != change.color_id
                && let Some(item) = item
            {
                *costs.entry(item).or_insert(0) += 1;
            }
        }
        Ok(costs)
    }
}
