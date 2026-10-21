//! TC a352b1fa SpellEffects.cpp1957–2024 and Object.cpp1191–1265:
//! non-special Wild/Ally titles create ordinary TempSummon, not a new control class.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::{ModelDisplay, Npc, UnitSummonedBy};

fn assert_default_source_actor(item: u32, spell: u32, entries: &[u32], lifetime_frames: usize) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("default-summon.redb"));
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, item);
    let caster = h.caster;
    let start = *h.app.world().get::<Position>(caster).unwrap();
    let destination = [start.x + 2.0, start.y, start.z + 1.0];
    assert_eq!(
        activate_at(&mut h, item, Some(destination))[0].error,
        None,
        "item{item}"
    );
    h.advance(40);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    let world = h.app.world_mut();
    let mut actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, _, summon)| summon.owner == caster && summon.spell_id == spell)
        .map(|(entity, npc, _)| (entity, npc.template_id))
        .collect();
    actors.sort_by_key(|(_, entry)| *entry);
    let mut expected = entries.to_vec();
    expected.sort();
    assert_eq!(
        actors.iter().map(|(_, entry)| *entry).collect::<Vec<_>>(),
        expected
    );
    for &(actor, entry) in &actors {
        let template = &world.resource::<crate::world_data::WorldData>().templates[&entry];
        assert!(
            template
                .models
                .iter()
                .any(|model| model.display_id
                    == world.get::<ModelDisplay>(actor).unwrap().display_id)
        );
        assert_eq!(
            world.get::<UnitSummonedBy>(actor).unwrap().0,
            caster.to_bits()
        );
        assert_eq!(
            world.get::<crate::maps::InMap>(actor),
            world.get::<crate::maps::InMap>(caster)
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(actor)
                .is_none()
        );
        assert!(
            !world
                .get::<SummonedCreature>(actor)
                .unwrap()
                .properties
                .is_guardian()
        );
    }
    h.advance(lifetime_frames - 50);
    for &(actor, _) in &actors {
        assert!(h.app.world().get_entity(actor).is_ok());
    }
    h.advance(80);
    for &(actor, _) in &actors {
        assert!(h.app.world().get_entity(actor).is_err());
    }
}

#[test]
fn toy_coverage_default_title_construct7_source_actor_and_eleven_minute_lifetime() {
    assert_default_source_actor(168667, 298926, &[153897], 13200);
}
#[test]
fn toy_coverage_default_title_name27_source_actor_and_five_minute_lifetime() {
    assert_default_source_actor(192485, 373230, &[189951], 6000);
}
#[test]
fn toy_coverage_default_title61_gilded_source_actor_and_five_minute_lifetime() {
    assert_default_source_actor(258135, 1262837, &[254652], 6000);
}
#[test]
fn toy_coverage_default_title61_azure_source_actor_and_five_minute_lifetime() {
    assert_default_source_actor(258136, 1262850, &[254655], 6000);
}
#[test]
fn toy_coverage_default_title_name27_two_gnomatic_source_actors_and_two_minute_lifetime() {
    assert_default_source_actor(267472, 1280448, &[259655, 259656], 2400);
}

#[test]
fn toy_coverage_default_title_census_retains_source_interaction_and_control_gaps() {
    let report = report_toy_census(
        &crate::test_data::world_db_path(),
        "TOY_DEFAULT_TITLE_CENSUS_REPORT",
    );
    let rows = report["toys"].as_array().unwrap();
    for item in [168667, 192485, 258135, 258136, 267472] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [198206, 198264, 210975, 37710, 54212, 87214, 111821] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
