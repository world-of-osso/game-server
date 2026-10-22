//! Test-only authentic profile/template loading through production follower spawning.
use bevy::prelude::*;

pub(crate) fn spawn(world: &mut World, owner: Entity, entry: u32) -> Result<Entity, String> {
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
