use super::*;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player,
};

#[test]
fn build_inspect_snapshot_includes_equipment_and_empty_legacy_talents() {
    let target = Player {
        name: "Alice".into(),
        race: 1,
        class: 2,
        appearance: CharacterAppearance::default(),
    };
    let equipment_appearance = EquipmentAppearance {
        entries: vec![EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: Some(100),
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: Some(200),
            inventory_type: 1,
            hidden: false,
        }],
    };

    let snapshot = build_inspect_snapshot(&target, &equipment_appearance);

    assert_eq!(snapshot.target_name, "Alice");
    assert_eq!(snapshot.equipment_appearance, equipment_appearance);
    assert!(snapshot.talents.talents.is_empty());
    assert!(snapshot.talents.spec_tabs.is_empty());
}

#[test]
fn inspect_target_entity_rejects_self_target() {
    let player = Entity::from_bits(1);
    let result = inspect_target_entity(player, Some(player), true);
    assert_eq!(result, Err("inspect target must be another player"));
}

#[test]
fn inspect_target_entity_rejects_non_player_target() {
    let player = Entity::from_bits(1);
    let target = Entity::from_bits(2);

    let result = inspect_target_entity(player, Some(target), false);

    assert_eq!(result, Err("inspect target is not a player"));
}
