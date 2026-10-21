//! Generic source dependency import and source-marker interaction admission.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::{ModelDisplay, Npc, UnitSummonedBy};

#[test]
fn toy_coverage_summon_marker_keeps_named_contracts_and_unproved_gossip_refused() {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = toy_catalog::load(&conn).unwrap();
    for (item, reason) in [
        (88387, "spittoon"),
        (88579, "brew"),
        (193478, "seating"),
        (198156, "wormhole"),
        (219387, "fireworks"),
        (118191, "npcflag 3"),
        (170203, "npcflag 16777216"),
        (69215, "gameobject effect 50"),
    ] {
        let unavailable = catalog.0[&item]
            .snapshot
            .unavailable_reason
            .as_deref()
            .unwrap_or("");
        assert!(unavailable.contains(reason), "item{item}: {unavailable:?}");
    }
    // Mailbox exemption is anchored by toyfx66_katy_real_learning_summon_mailbox_service_and_source_expiry.
    assert!(catalog.0[&156833].snapshot.unavailable_reason.is_none());
    assert!(catalog.0[&186501].snapshot.unavailable_reason.is_none());
}

#[test]
fn toy_coverage_summon_closure_census_preserves_marked_interaction_gaps() {
    let report = report_toy_census(&toyfx8_world(), "TOY_COVERAGE_SUMMON_REPORT");
    for item in [88387, 88579, 193478, 198156, 219387, 118191] {
        let row = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["item_id"] == item)
            .unwrap();
        assert!(
            !row["unavailable"].is_null(),
            "interaction item{item}: {row}"
        );
    }
    for item in [64358, 80822, 186501, 156833] {
        let row = report["toys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["item_id"] == item)
            .unwrap();
        assert!(
            row["unavailable"].is_null(),
            "actor dependency/exempt service item{item}: {row}"
        );
    }
}

#[test]
fn toy_coverage_summon_closure_actual_trophy_use_spawns_both_authored_actors_and_expires() {
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("trophy.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, 186501);
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let destination = Position {
        x: position.x + 2.0,
        y: position.y,
        z: position.z + 1.0,
    };
    assert_eq!(
        activate_at(
            &mut h,
            186501,
            Some([destination.x, destination.y, destination.z])
        )[0]
        .error,
        None
    );
    h.advance(10);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 353904));
    let caster = h.caster;
    let world = h.app.world_mut();
    let mut actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, _, summon)| summon.owner == caster && summon.spell_id == 353904)
        .map(|(entity, npc, _)| (entity, npc.template_id))
        .collect();
    actors.sort_by_key(|(_, entry)| *entry);
    assert_eq!(
        actors.iter().map(|(_, entry)| *entry).collect::<Vec<_>>(),
        [179325, 180947]
    );
    for ((entity, _), display) in actors.iter().zip([101290, 102004]) {
        assert_eq!(
            world.get::<ModelDisplay>(*entity).unwrap().display_id,
            display
        );
        assert_eq!(
            world.get::<UnitSummonedBy>(*entity).unwrap().0,
            caster.to_bits()
        );
        assert_eq!(world.get::<Position>(*entity), Some(&destination));
        assert_eq!(
            world.get::<crate::maps::InMap>(*entity),
            world.get::<crate::maps::InMap>(caster)
        );
        assert_eq!(
            world
                .get::<SummonedCreature>(*entity)
                .unwrap()
                .display_scale,
            0.5
        );
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(*entity)
                .is_none()
        );
    }
    h.advance(1170);
    for (actor, _) in &actors {
        assert!(h.app.world().get_entity(*actor).is_ok());
    }
    h.advance(40);
    for (actor, _) in &actors {
        assert!(h.app.world().get_entity(*actor).is_err());
    }
    assert_eq!(
        activate_at(
            &mut h,
            186501,
            Some([destination.x, destination.y, destination.z])
        )[0]
        .error
        .as_deref(),
        Some("OnCooldown")
    );
}
