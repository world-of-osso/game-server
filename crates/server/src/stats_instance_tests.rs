//! Native item-instance changes must traverse the live StatsPlugin change boundary.
use super::*;
use rusqlite::{Connection, OpenFlags};
use server::item_instance::{ItemInstanceModifiers, load};
use shared::item_data::ItemDefinitionSource;

#[test]
fn item_instance_native_equipped_enchant_change_recomputes_character_stats() {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::test_data::world_db_path);
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (spells, classes) = crate::spell_info::load_spell_data(&conn).unwrap();
    let native = NativeItemInstances {
        bonuses: load::load_bonus_tables(&conn).unwrap(),
        enchants: load::load_enchantment_tables(&conn).unwrap(),
        effects: load::load_item_effect_tables(&conn).unwrap(),
    };
    let mut app = App::new();
    app.insert_resource(spells)
        .insert_resource(classes)
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap())
        .insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap())
        .insert_resource(native)
        .add_plugins(StatsPlugin);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "ParityPaladin".into(),
                class: 2,
                race: 6,
                appearance: Default::default(),
            },
            Level(90),
            ActiveSpec(66),
            EquippedItems {
                items: vec![(EquipmentSlot::Chest, ItemDefinitionSource::Retail, 271468)],
            },
            EquippedItemModifiers {
                items: vec![(
                    EquipmentSlot::Chest,
                    ItemInstanceModifiers {
                        item_level_override: Some(344),
                        ..Default::default()
                    },
                )],
            },
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            Health {
                current: 1.0,
                max: 1.0,
            },
        ))
        .id();
    app.update();
    let before = *app.world().get::<CharacterStats>(player).unwrap();
    app.update();
    app.world_mut()
        .get_mut::<EquippedItemModifiers>(player)
        .unwrap()
        .items[0]
        .1
        .enchant = 7987;
    app.update();
    let after = *app.world().get::<CharacterStats>(player).unwrap();
    assert_eq!(
        after.primary.strength,
        before.primary.strength + 50.0,
        "native344 chest plus actual7987 must reach the player after a modifier-only change"
    );
    assert!(after.attack_power > before.attack_power);
}
