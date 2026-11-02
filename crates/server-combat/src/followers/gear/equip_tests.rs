//! Real source equipment through the ordinary follower stat recomputation.
use super::*;
use crate::stats::{
    CharacterStats, EquippedItemModifiers, EquippedItems, NativeItemInstances, StatsPlugin,
};

fn equipped_follower(entry: u32, level: u8, map: u32, difficulty: u32) -> (f32, f32, Vec<u16>) {
    let (mut h, actor) = super::super::kit_tests::at_level_unengaged(entry, level, |app| {
        app.add_plugins(StatsPlugin);
    });
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::item_proc_data::load(&db).unwrap());
    let source = load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap();
    h.app.insert_resource(source);
    h.app
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    h.app
        .insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    h.app.insert_resource(NativeItemInstances {
        bonuses: crate::item_instance::load::load_bonus_tables(&db).unwrap(),
        enchants: crate::item_instance::load::load_enchantment_tables(&db).unwrap(),
        effects: crate::item_instance::load::load_item_effect_tables(&db).unwrap(),
    });
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            map,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: difficulty,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(instances);
    h.app.world_mut().entity_mut(actor).insert(copy);
    h.advance(1);
    let before = h
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .spell_power;
    let profile = h
        .app
        .world()
        .get::<super::super::Follower>(actor)
        .unwrap()
        .profile
        .clone();
    equip(h.app.world_mut(), actor, &profile, &[]).unwrap();
    h.advance(1);
    let after = h
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .spell_power;
    let items = h.app.world().get::<EquippedItems>(actor).unwrap();
    let modifiers = h.app.world().get::<EquippedItemModifiers>(actor).unwrap();
    assert!(!items.items.is_empty(), "follower equipment is populated");
    let levels = modifiers
        .items
        .iter()
        .map(|(_, value)| value.item_level_override.unwrap())
        .collect();
    (before, after, levels)
}

#[test]
fn follower_gear_equips_real_items_and_stats_follow_dungeon_difficulty() {
    let (before, normal, normal_levels) = equipped_follower(209059, 70, 2515, 205);
    let (_, mythic, mythic_levels) = equipped_follower(209059, 70, 2515, 23);
    assert_eq!(normal_levels.len(), 16, "all mage equipment slots filled");
    assert!(normal_levels.iter().all(|level| *level == 73));
    assert!(mythic_levels.iter().all(|level| *level == 76));
    assert!(normal > before, "real equipment increases spell power");
    assert!(
        mythic > normal,
        "sourced difficulty level changes the same follower's stats"
    );
}

/// Every LFGDungeons follower entry (difficulty205) has a MapDifficulty row whose
/// ItemContext/ItemContextPickerID resolves at both ContentTuning level bounds.
#[test]
fn every_follower_dungeon_resolves_its_item_level_at_both_level_bounds() {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let source = load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let scaling = crate::item_stats::load_item_scaling_tables(&path).unwrap();
    let bonuses = crate::item_instance::load::load_bonus_tables(&db).unwrap();
    let dungeons: Vec<(u32, u8, u8)> = db
        .prepare(
            "SELECT d.map_id,t.MinLevelSquish,t.MaxLevelSquish FROM follower_lfg_dungeons d
             JOIN content_tuning t ON t.ID=d.content_tuning_id WHERE d.difficulty_id=205",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(dungeons.len(), 27, "every follower LFG dungeon is covered");
    let failures: Vec<_> = dungeons
        .iter()
        .flat_map(|&(map, min, max)| [(map, min), (map, max)])
        .filter_map(|(map, level)| {
            source
                .read_item_level(map, 205, level, &templates, &bonuses, &scaling)
                .err()
                .map(|error| format!("map{map} level{level}: {error}"))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn murder_row_level_90_followers_equip_the_same_sourced_difficulty_level() {
    let mut dungeon_level = None;
    for entry in [209057, 209072, 209053, 209059] {
        let (_, _, levels) = equipped_follower(entry, 90, 2813, 205);
        let level = levels[0];
        assert!(levels.iter().all(|value| *value == level));
        assert_eq!(*dungeon_level.get_or_insert(level), level);
        eprintln!(
            "follower {entry}: {} slots at item level {level}",
            levels.len()
        );
    }
}
