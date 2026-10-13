//! Authored demon summon from an NPC companion, with no character/stable writes.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use redb::{ReadableDatabase, ReadableTableMetadata};

fn count_persisted_rows(world: &bevy::prelude::World) -> u64 {
    let db = world.resource::<crate::persistence::GameDatabase>();
    let read = db.db.begin_read().unwrap();
    read.list_tables()
        .unwrap()
        .map(|table| read.open_untyped_table(table).unwrap().len().unwrap())
        .sum()
}

#[test]
fn follower_lantha_summons_authored_felguard_without_persisting_a_character() {
    let prepared = crate::spell_cast::test_harness::prepare_class_world();
    let mut data = crate::world_data::load_world_data(&prepared.path());
    data.templates.retain(|entry, _| *entry == 17252);
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let spells = crate::pet_control::load_pet_levelup_spells(&source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("npc-pets.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(230861, |app| {
        app.insert_resource(data);
        app.insert_resource(spells);
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.add_plugins(crate::hunter_pet::HunterPetPlugin);
    });
    let before = count_persisted_rows(h.app.world());
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 30146, Some(unit), false)
        .expect("authored companion demon summon");
    h.advance(160);
    let pet =
        crate::hunter_pet::summoned_pet(h.app.world(), unit).expect("Felguard summoned by Lantha");
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(pet)
            .unwrap()
            .0,
        unit.to_bits()
    );
    assert_eq!(
        count_persisted_rows(h.app.world()),
        before,
        "runtime NPC pet must not write a persistent character or stable"
    );
}
