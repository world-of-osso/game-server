use super::*;
use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::stats::{EquippedItemModifiers, EquippedItems, NativeItemInstances};
use rusqlite::{Connection, OpenFlags};

#[test]
fn simc_profile_and_journal_install_identical_loadouts() {
    let (mut h, _, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let source = crate::test_data::world_data();
    let mut world_data = h
        .app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>();
    world_data.class_level_base_hp = source.class_level_base_hp.clone();
    world_data.class_level_base_mana = source.class_level_base_mana.clone();
    drop(world_data);
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(super::super::catalog::load_catalog(&db).unwrap());
    h.app.insert_resource(
        super::super::gear::load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap(),
    );
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
                difficulty_id: 205,
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
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((copy, Level(90)));
    h.app.insert_resource(super::super::FollowerActionProvider {
        before_passives: |_, _, _| Ok(()),
        source_actions: |_, _, _| Ok(vec![30451]),
        equipped_actions: |_, _, _| Ok(Vec::new()),
    });
    let mut profile = super::super::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 62)
        .unwrap();
    profile.race = 1;
    let journal = spawn_class(
        h.app.world_mut(),
        h.caster,
        &profile,
        0,
        0,
        LoadoutSource::Journal,
    )
    .unwrap();
    let world = h.app.world();
    let items = super::super::parity_profile::ItemInstances {
        items: world.get::<EquippedItems>(journal).unwrap().items.clone(),
        modifiers: world.get::<EquippedItemModifiers>(journal).unwrap().clone(),
    };
    let traits = world
        .get::<crate::class_progression::TraitLoadout>(journal)
        .unwrap()
        .source_config()
        .0
        .iter()
        .filter(|entry| entry.total() > 0)
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.total(),
        })
        .collect();
    let simc = spawn_class(
        h.app.world_mut(),
        h.caster,
        &profile,
        0,
        1,
        LoadoutSource::SimcProfile {
            items,
            traits,
            pet: None,
        },
    )
    .unwrap();
    for _ in 0..10 {
        h.app.update();
    }
    let world = h.app.world();
    assert_eq!(
        world.get::<EquippedItems>(journal),
        world.get::<EquippedItems>(simc)
    );
    assert_eq!(
        world.get::<EquippedItemModifiers>(journal),
        world.get::<EquippedItemModifiers>(simc)
    );
    assert_eq!(
        world.get::<crate::stats::CharacterStats>(journal),
        world.get::<crate::stats::CharacterStats>(simc)
    );
    let spells = |unit| {
        world
            .get::<crate::class_progression::KnownSpells>(unit)
            .unwrap()
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(world.get::<CharacterVisual>(journal).unwrap().race, 1);
    assert_eq!(world.get::<CharacterVisual>(simc).unwrap().race, 1);
    assert!(spells(journal).contains(&59752));
    assert!(spells(journal).contains(&20598));
    assert_eq!(spells(journal), spells(simc));
    let auras = |unit| {
        world
            .get::<crate::auras::ActiveAuras>(unit)
            .unwrap()
            .iter()
            .map(|aura| (aura.spell_id, aura.stacks))
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert!(auras(journal).iter().any(|(spell, _)| *spell == 20598));
    assert_eq!(auras(journal), auras(simc));
}

#[test]
fn journal_wcl_baseline_snapshots() {
    let (mut h, _, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let source = crate::test_data::world_data();
    let mut world_data = h
        .app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>();
    world_data.class_level_base_hp = source.class_level_base_hp.clone();
    world_data.class_level_base_mana = source.class_level_base_mana.clone();
    drop(world_data);
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::item_proc_data::load(&db).unwrap());
    h.app
        .insert_resource(super::super::catalog::load_catalog(&db).unwrap());
    h.app.insert_resource(
        super::super::gear::load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap(),
    );
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
                difficulty_id: 205,
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
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((copy, Level(90)));
    h.app.insert_resource(super::super::FollowerActionProvider {
        before_passives: |_, _, _| Ok(()),
        source_actions: |_, _, _| Ok(vec![30451]),
        equipped_actions: |_, _, _| Ok(Vec::new()),
    });
    let mut profile = super::super::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 62)
        .unwrap();
    profile.race = 1;
    for (name, gear) in [
        ("journal", GearSource::Journal),
        ("wcl", GearSource::KeystoneBand(11)),
    ] {
        let unit = spawn_class(h.app.world_mut(), h.caster, &profile, 0, 0, gear).unwrap();
        for _ in 0..10 {
            h.app.update();
        }
        let actual = format!(
            "{:?}",
            h.app
                .world()
                .get::<crate::stats::CharacterStats>(unit)
                .unwrap()
        );
        let expected = match name {
            "journal" => include_str!("snapshots/journal.txt"),
            "wcl" => include_str!("snapshots/wcl.txt"),
            _ => unreachable!(),
        };
        assert_eq!(actual, expected.trim_end(), "{name} stats changed");
    }
}

#[test]
fn same_simc_loadout_installs_same_pet_for_spawn_and_install() {
    let (mut h, _, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let source = crate::test_data::world_data();
    let mut world_data = h
        .app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>();
    let pets = crate::spell_cast::test_harness::load_native_creature_world(&[154, 174884]);
    world_data.templates.extend(pets.templates);
    world_data.class_level_base_hp = source.class_level_base_hp.clone();
    world_data.class_level_base_mana = source.class_level_base_mana.clone();
    drop(world_data);
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(crate::hunter_pet::read_pet_catalog(&db).unwrap());
    h.app
        .insert_resource(super::super::catalog::load_catalog(&db).unwrap());
    h.app.insert_resource(
        super::super::gear::load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap(),
    );
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
                difficulty_id: 205,
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
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((copy, Level(90)));
    h.app.insert_resource(super::super::FollowerActionProvider {
        before_passives: |_, _, _| Ok(()),
        source_actions: |_, _, _| Ok(vec![75]),
        equipped_actions: |_, _, _| Ok(Vec::new()),
    });
    let profile = super::super::class_profiles::read_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.spec_id == 253)
        .unwrap();
    let journal = spawn_class(
        h.app.world_mut(),
        h.caster,
        &profile,
        0,
        0,
        LoadoutSource::Journal,
    )
    .unwrap();
    let world = h.app.world();
    let items = super::super::parity_profile::ItemInstances {
        items: world.get::<EquippedItems>(journal).unwrap().items.clone(),
        modifiers: world.get::<EquippedItemModifiers>(journal).unwrap().clone(),
    };
    let traits: Vec<_> = world
        .get::<crate::class_progression::TraitLoadout>(journal)
        .unwrap()
        .source_config()
        .0
        .iter()
        .filter(|entry| entry.total() > 0)
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.total(),
        })
        .collect();
    let spawned = spawn_class(
        h.app.world_mut(),
        h.caster,
        &profile,
        0,
        1,
        LoadoutSource::SimcProfile {
            items: items.clone(),
            traits: traits.clone(),
            pet: Some(174884),
        },
    )
    .unwrap();
    let installed = spawn_neutral(h.app.world_mut(), h.caster, "installed", 90).unwrap();
    h.app
        .world_mut()
        .entity_mut(installed)
        .insert(CharacterVisual {
            race: profile.race,
            class: profile.class_id,
            appearance: CharacterAppearance::default(),
        });
    install_class(
        h.app.world_mut(),
        h.caster,
        installed,
        &profile,
        0,
        2,
        LoadoutSource::SimcProfile {
            items,
            traits,
            pet: Some(174884),
        },
    )
    .unwrap();
    let world = h.app.world();
    let pet = |actor| {
        world
            .get::<crate::hunter_pet::PetStable>(actor)
            .unwrap()
            .stored
            .active[0]
            .as_ref()
            .unwrap()
            .creature_entry
    };
    assert_eq!(pet(journal), 154);
    assert_eq!(pet(spawned), 174884);
    assert_eq!(pet(installed), pet(spawned));
}
