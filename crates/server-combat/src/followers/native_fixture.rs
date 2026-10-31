//! Test-only authentic profile/template loading through production follower spawning.
use bevy::prelude::*;

pub fn spawn(world: &mut World, owner: Entity, entry: u32) -> Result<Entity, String> {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("follower fixture database: {error}"))?;
    let catalog = super::catalog::load_catalog(&db)
        .map_err(|error| format!("follower fixture catalog: {error}"))?;
    let profile = catalog
        .profiles
        .iter()
        .find(|profile| profile.entry == entry)
        .cloned()
        .ok_or_else(|| format!("native follower profile {entry} absent"))?;
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[entry]);
    {
        let mut data = world.resource_mut::<crate::world_data::WorldData>();
        data.templates.extend(native.templates);
        data.difficulty_templates
            .extend(native.difficulty_templates);
        let source = crate::test_data::world_data();
        data.class_level_base_hp = source.class_level_base_hp.clone();
        data.class_level_base_mana = source.class_level_base_mana.clone();
        data.armor_constants = source.armor_constants.clone();
    }
    world.insert_resource(
        crate::spell_info::load_class_spells(&db)
            .map_err(|error| format!("follower fixture classes: {error}"))?,
    );
    world.insert_resource(catalog);
    super::spawning::spawn(world, owner, profile)
}

/// Install the world sources a class follower spawn reads: native classes,
/// base pools, Journal gear and item data, loaded as server startup loads them.
pub fn install_class_sources(world: &mut World) {
    let path = crate::test_data::world_db_path();
    let db =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("class follower fixture database");
    {
        let source = crate::test_data::world_data();
        let mut data = world.resource_mut::<crate::world_data::WorldData>();
        data.class_level_base_hp = source.class_level_base_hp.clone();
        data.class_level_base_mana = source.class_level_base_mana.clone();
        data.armor_constants = source.armor_constants.clone();
    }
    world.insert_resource(crate::spell_info::load_class_spells(&db).unwrap());
    world.insert_resource(
        super::gear::load_source(&path, &path.parent().unwrap().join("db2/active")).unwrap(),
    );
    world.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    world.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    world.insert_resource(crate::stats::NativeItemInstances {
        bonuses: crate::item_instance::load::load_bonus_tables(&db).unwrap(),
        enchants: crate::item_instance::load::load_enchantment_tables(&db).unwrap(),
        effects: crate::item_instance::load::load_item_effect_tables(&db).unwrap(),
    });
}

/// Spawn the roster's `spec` class follower for `owner` through production
/// `spawn_class`, with Journal gear from the owner's copy.
pub fn spawn_class(world: &mut World, owner: Entity, spec: u32) -> Result<Entity, String> {
    install_class_sources(world);
    let profile = super::class_profiles::read_profiles()?
        .into_iter()
        .find(|profile| profile.spec_id == spec)
        .ok_or_else(|| format!("class follower spec{spec} absent"))?;
    let gear = super::class_spawn::GearSource::Journal;
    super::class_spawn::spawn_class(world, owner, &profile, 0, 0, gear)
}

/// The fixture spawns without zone visuals, so read the authentic template display.
#[cfg(test)]
pub(crate) fn template_display(entry: u32) -> u32 {
    rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row(
        "SELECT CreatureDisplayID FROM content_creature_template_model
         WHERE CreatureID=?1 ORDER BY Idx LIMIT 1",
        [entry],
        |row| row.get::<_, u32>(0),
    )
    .expect("authentic follower template display")
}
