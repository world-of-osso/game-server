//! Per-instance appearance modifiers, separate from existing item/character ABIs.
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};
use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::TransmogChoice;
use std::collections::HashMap;

const ITEM_TRANSMOGS: TableDefinition<u64, &[u8]> = TableDefinition::new("item_transmogs");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItemTransmog {
    pub source_id: u32,
    pub display_id: u32,
    pub inventory_type: u8,
    pub hidden: bool,
}

impl ItemTransmog {
    pub(crate) fn choice(self) -> TransmogChoice {
        if self.hidden {
            TransmogChoice::Hidden
        } else {
            TransmogChoice::Source(self.source_id)
        }
    }

    pub(crate) fn entry(
        self,
        slot: EquipmentVisualSlot,
        item_id: u32,
        definition_source: ItemDefinitionSource,
    ) -> EquippedAppearanceEntry {
        EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            definition_source: Some(definition_source),
            display_info_id: Some(self.display_id),
            inventory_type: self.inventory_type,
            hidden: self.hidden,
        }
    }
}

pub(crate) fn load_item_transmogs(
    db: &Database,
    guids: impl Iterator<Item = u64>,
) -> HashMap<u64, ItemTransmog> {
    let txn = db.begin_read().expect("begin transmog read");
    let table = match txn.open_table(ITEM_TRANSMOGS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return HashMap::new(),
        Err(error) => panic!("open item transmogs: {error}"),
    };
    guids
        .filter_map(|guid| {
            table.get(guid).expect("read item transmog").map(|row| {
                (
                    guid,
                    bitcode::decode(row.value()).expect("decode item transmog"),
                )
            })
        })
        .collect()
}

pub(crate) fn write_item_transmogs(
    txn: &WriteTransaction,
    owned: impl Iterator<Item = u64>,
    overrides: &HashMap<u64, ItemTransmog>,
) {
    let mut table = txn.open_table(ITEM_TRANSMOGS).expect("open item transmogs");
    for guid in owned {
        match overrides.get(&guid) {
            Some(value) => {
                table
                    .insert(guid, bitcode::encode(value).as_slice())
                    .expect("write item transmog");
            }
            None => {
                table.remove(guid).expect("remove item transmog");
            }
        }
    }
}
