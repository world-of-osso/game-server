//! Player-character presentation for non-player follower entities.
use bevy::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use shared::components::{
    CharacterVisual, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
    ModelDisplay,
};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

fn visual_slot(slot: EquipmentSlot) -> Option<EquipmentVisualSlot> {
    use EquipmentSlot as Slot;
    use EquipmentVisualSlot as Visual;
    match slot {
        Slot::Head => Some(Visual::Head),
        Slot::Shoulder => Some(Visual::Shoulder),
        Slot::Back => Some(Visual::Back),
        Slot::Chest => Some(Visual::Chest),
        Slot::Shirt => Some(Visual::Shirt),
        Slot::Tabard => Some(Visual::Tabard),
        Slot::Wrist => Some(Visual::Wrist),
        Slot::Hands => Some(Visual::Hands),
        Slot::Waist => Some(Visual::Waist),
        Slot::Legs => Some(Visual::Legs),
        Slot::Feet => Some(Visual::Feet),
        Slot::MainHand => Some(Visual::MainHand),
        Slot::OffHand => Some(Visual::OffHand),
        Slot::Ranged => Some(Visual::Ranged),
        _ => None,
    }
}

/// Read default item appearances for the actual equipped visual slots.
pub fn read_equipment_appearance(
    db: &Connection,
    equipped: &crate::stats::EquippedItems,
) -> Result<EquipmentAppearance, String> {
    let mut entries = Vec::new();
    for &(slot, source, item_id) in &equipped.items {
        let Some(slot) = visual_slot(slot) else {
            continue;
        };
        if source != ItemDefinitionSource::Retail {
            return Err(format!(
                "unsupported follower item source {source:?} for item {item_id}"
            ));
        }
        let row: Option<(u8, u32)> = db.query_row(
            "SELECT i.InventoryType, a.ItemDisplayInfoID FROM item i JOIN item_modified_appearance m ON m.ItemID=i.ID JOIN item_appearance a ON a.ID=m.ItemAppearanceID WHERE i.ID=?1 AND a.ItemDisplayInfoID>0 ORDER BY m.ItemAppearanceModifierID, m.ID LIMIT 1",
            [item_id], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|error| format!("read follower item {item_id} appearance: {error}"))?;
        let (inventory_type, display_info_id) =
            row.ok_or_else(|| format!("follower item {item_id} has no appearance"))?;
        entries.push(EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            definition_source: Some(source),
            display_info_id: Some(display_info_id),
            inventory_type,
            hidden: false,
        });
    }
    Ok(EquipmentAppearance { entries })
}

/// Replace the creature display with the given character and equipment appearance.
pub fn publish(
    world: &mut World,
    entity: Entity,
    visual: CharacterVisual,
    equipment: EquipmentAppearance,
) {
    world
        .entity_mut(entity)
        .remove::<ModelDisplay>()
        .insert((visual, equipment));
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::{CharacterAppearance, Player};

    #[test]
    fn follower_visual_uses_equipped_items_without_player_or_npc_display() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE item(ID INTEGER, InventoryType INTEGER); CREATE TABLE item_modified_appearance(ID INTEGER, ItemID INTEGER, ItemAppearanceID INTEGER, ItemAppearanceModifierID INTEGER); CREATE TABLE item_appearance(ID INTEGER, ItemDisplayInfoID INTEGER); INSERT INTO item VALUES(123, 1), (456, 13); INSERT INTO item_modified_appearance VALUES(1, 123, 11, 0), (2, 456, 12, 0); INSERT INTO item_appearance VALUES(11, 101), (12, 202);").unwrap();
        let equipped = crate::stats::EquippedItems {
            items: vec![
                (EquipmentSlot::Head, ItemDefinitionSource::Retail, 123),
                (EquipmentSlot::MainHand, ItemDefinitionSource::Retail, 456),
            ],
        };
        let equipment = read_equipment_appearance(&db, &equipped).unwrap();
        let mut world = World::new();
        let entity = world.spawn(ModelDisplay { display_id: 999 }).id();
        let visual = CharacterVisual {
            race: 3,
            class: 2,
            appearance: CharacterAppearance {
                hair_style: 4,
                ..Default::default()
            },
        };
        publish(&mut world, entity, visual.clone(), equipment);
        assert_eq!(world.get::<CharacterVisual>(entity), Some(&visual));
        assert!(world.get::<Player>(entity).is_none());
        assert!(world.get::<ModelDisplay>(entity).is_none());
        let entries = &world.get::<EquipmentAppearance>(entity).unwrap().entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(
            (entries[0].item_id, entries[0].display_info_id),
            (Some(123), Some(101))
        );
        assert_eq!(
            (entries[1].item_id, entries[1].display_info_id),
            (Some(456), Some(202))
        );
    }
}
