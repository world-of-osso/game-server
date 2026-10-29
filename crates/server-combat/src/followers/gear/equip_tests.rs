//! Real source equipment through the ordinary follower stat recomputation.
use super::*;
use crate::stats::{
    CharacterStats, EquippedItemModifiers, EquippedItems, NativeItemInstances, StatsPlugin,
};

fn equipped_fire_mage(difficulty: u32) -> (f32, f32, Vec<u16>) {
    let (mut h, actor) = super::super::kit_tests::at_level_unengaged(209059, 70, |app| {
        app.add_plugins(StatsPlugin);
    });
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
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
            2515,
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
    equip(h.app.world_mut(), actor, &profile).unwrap();
    h.advance(1);
    let after = h
        .app
        .world()
        .get::<CharacterStats>(actor)
        .unwrap()
        .spell_power;
    let items = h.app.world().get::<EquippedItems>(actor).unwrap();
    let modifiers = h.app.world().get::<EquippedItemModifiers>(actor).unwrap();
    assert_eq!(items.items.len(), 16, "all mage equipment slots filled");
    let levels = modifiers
        .items
        .iter()
        .map(|(_, value)| value.item_level_override.unwrap())
        .collect();
    (before, after, levels)
}

#[test]
fn follower_gear_equips_real_items_and_stats_follow_dungeon_difficulty() {
    let (before, normal, normal_levels) = equipped_fire_mage(205);
    let (_, mythic, mythic_levels) = equipped_fire_mage(23);
    assert!(normal_levels.iter().all(|level| *level == 73));
    assert!(mythic_levels.iter().all(|level| *level == 76));
    assert!(normal > before, "real equipment increases spell power");
    assert!(
        mythic > normal,
        "sourced difficulty level changes the same follower's stats"
    );
}
