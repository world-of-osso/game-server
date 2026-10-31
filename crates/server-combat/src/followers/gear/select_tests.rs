//! Real active-data selection for every authenticated follower specialization.
use super::*;
use rusqlite::{Connection, OpenFlags};

#[test]
fn follower_gear_selects_all_native_dragonflight_slots_for_every_spec() {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let db2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let pools = super::super::read_dungeon_item_pools(&db, &db2).unwrap();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
    let item_specs = super::super::item_spec::ItemSpecs::load(&db).unwrap();
    let profiles = crate::followers::catalog::load_catalog(&db).unwrap();
    for profile in profiles.profiles.iter() {
        let class = classes
            .classes
            .iter()
            .find(|class| class.class_id == profile.class_id)
            .unwrap();
        let spec = class
            .specs
            .iter()
            .find(|spec| spec.spec_id == profile.spec_id)
            .unwrap();
        let primary = PrimaryStat::from_priority(spec.primary_stat_priority);
        let (book, _) = crate::class_progression::evaluate_npc_starter(
            &classes,
            profile.class_id,
            profile.spec_id,
            70,
        )
        .unwrap();
        let known = KnownSpells::from_baseline(book.iter().map(|spell| spell.spell_id).collect());
        let gear = select_items(
            &pools[&2515],
            2515,
            profile.class_id,
            primary,
            profile.spec_id,
            70,
            &known,
            &spells,
            &templates,
            &item_specs,
        );
        eprintln!(
            "GEAR spec={} items={:?} missing={:?}",
            profile.spec_id, gear.items, gear.missing
        );
        if matches!(profile.spec_id, 66 | 73) {
            let shield = gear
                .items
                .iter()
                .find(|(slot, _, _)| *slot == EquipmentSlot::OffHand)
                .unwrap();
            assert_eq!(
                templates.get(shield.1, shield.2).unwrap().inventory_type,
                14,
                "protection uses a shield"
            );
        }
        if profile.spec_id == 253 {
            let weapon = gear
                .items
                .iter()
                .find(|(slot, _, _)| *slot == EquipmentSlot::MainHand)
                .unwrap();
            assert!(
                matches!(
                    templates.get(weapon.1, weapon.2).unwrap().inventory_type,
                    15 | 26
                ),
                "hunter uses a ranged weapon"
            );
        }
        assert!(
            gear.missing.is_empty(),
            "follower{} has missing source slots {:?}",
            profile.spec_id,
            gear.missing
        );
    }
}

#[test]
fn follower_weapons_follow_native_loot_specializations() {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let db2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let pools = super::super::read_dungeon_item_pools(&db, &db2).unwrap();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let item_specs = super::super::item_spec::ItemSpecs::load(&db).unwrap();
    let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
    let profiles = crate::followers::catalog::load_catalog(&db).unwrap();
    let dagger = templates
        .get(ItemDefinitionSource::Retail, 251_128)
        .unwrap();
    assert!(!item_specs.allows(251_128, dagger, 581, 90));
    for level in [70, 90] {
        let map = if level == 70 { 2515 } else { 2813 };
        for profile in &profiles.profiles {
            let class = classes
                .classes
                .iter()
                .find(|class| class.class_id == profile.class_id)
                .unwrap();
            let spec = class
                .specs
                .iter()
                .find(|spec| spec.spec_id == profile.spec_id)
                .unwrap();
            let (book, _) = crate::class_progression::evaluate_npc_starter(
                &classes,
                profile.class_id,
                profile.spec_id,
                level,
            )
            .unwrap();
            let known =
                KnownSpells::from_baseline(book.iter().map(|spell| spell.spell_id).collect());
            let selection = select_items(
                &pools[&map],
                map,
                profile.class_id,
                PrimaryStat::from_priority(spec.primary_stat_priority),
                profile.spec_id,
                level,
                &known,
                &spells,
                &templates,
                &item_specs,
            );
            let main = selection
                .items
                .iter()
                .find(|(slot, _, _)| *slot == EquipmentSlot::MainHand)
                .expect("class follower has a main-hand item");
            for (_, source, id) in selection.items.iter().filter(|(slot, _, _)| {
                matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand)
            }) {
                let item = templates.get(*source, *id).unwrap();
                if item.scaling.class_id == 2 {
                    assert!(
                        item_specs.allows(*id, item, profile.spec_id, level),
                        "spec{} level{level} item{id}",
                        profile.spec_id
                    );
                }
            }
            if profile.spec_id == 581 {
                assert_ne!(
                    main.2, 251_128,
                    "Vengeance must not receive the Devourer dagger"
                );
            }
        }
    }
}

/// Druids neither dual wield nor use shields. In tiers whose first eligible
/// weapon is a one-hander they wield a two-hander instead, never a missing slot.
#[test]
fn druid_followers_take_a_two_hander_when_no_off_hand_is_usable() {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let db2 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let pools = super::super::read_dungeon_item_pools(&db, &db2).unwrap();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
    let item_specs = super::super::item_spec::ItemSpecs::load(&db).unwrap();
    let druid = classes
        .classes
        .iter()
        .find(|class| class.class_id == 11)
        .unwrap();
    // Uldaman (DF), Operation: Floodgate (TWW), Den of Nalorakk (Midnight).
    for map in [2451, 2773, 2825] {
        for spec in [103, 104] {
            let primary = druid.specs.iter().find(|row| row.spec_id == spec).unwrap();
            let primary = PrimaryStat::from_priority(primary.primary_stat_priority);
            let (book, _) =
                crate::class_progression::evaluate_npc_starter(&classes, 11, spec, 70).unwrap();
            let known =
                KnownSpells::from_baseline(book.iter().map(|spell| spell.spell_id).collect());
            let gear = select_items(
                &pools[&map],
                map,
                11,
                primary,
                spec,
                70,
                &known,
                &spells,
                &templates,
                &item_specs,
            );
            assert!(
                gear.missing.is_empty(),
                "map{map} spec{spec} missing {:?}",
                gear.missing
            );
            let weapon = |slot| {
                gear.items
                    .iter()
                    .find(|(row, _, _)| *row == slot)
                    .map(|(_, source, item)| templates.get(*source, *item).unwrap())
            };
            let main = weapon(EquipmentSlot::MainHand).expect("druid main hand");
            assert!(
                matches!(main.inventory_type, 17) || weapon(EquipmentSlot::OffHand).is_some(),
                "map{map} spec{spec} wields a one-hander {} with nothing in the off hand",
                main.inventory_type
            );
        }
    }
}
