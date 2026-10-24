//! Real native proc events across generated/refused and explicit-TDB parents.
use super::kit_tests::at_level;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, Position};

fn fixture(parent: u32, explicit: bool) -> (Harness, Entity) {
    let (mut h, actor) = at_level(232800, 90, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 1.5;
    // Keep the ordinary hostile target alive; this does not modify spell damage.
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    assert!(crate::auras::has_aura(h.app.world(), actor, parent));
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let rows: u32 = db
        .query_row(
            "SELECT COUNT(*) FROM tdb_spell_proc WHERE SpellId=?",
            [parent],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rows != 0, explicit);
    (h, actor)
}

fn cast_tigers_fury(h: &mut Harness, actor: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, 5217, Some(actor), false)
        .expect("ordinary native Tiger's Fury cast");
    h.advance(3);
}

#[test]
#[ignore = "native private DB: TC generated-proc refusal after explicit provenance import"]
fn generic_proc_definition_refuses_unsourced_savage_fury() {
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor) = fixture(449645, false);
    assert!(!crate::auras::has_aura(h.app.world(), actor, 449646));
    cast_tigers_fury(&mut h, actor);
    assert!(crate::auras::has_aura(h.app.world(), actor, 5217));
    assert!(crate::auras::has_aura(h.app.world(), actor, 449645));
    assert!(
        !crate::auras::has_aura(h.app.world(), actor, 449646),
        "unsafe generated449645 must not dispatch449646 without explicit TDB provenance"
    );
}

#[test]
#[ignore = "native private DB: explicit48484 must survive generic default refusal"]
fn generic_proc_definition_preserves_explicit_infected_wounds() {
    use crate::spell_cast::CastFailure;
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor) = fixture(48484, true);
    let target = h.npc;
    assert!(!crate::auras::has_aura(h.app.world(), target, 58180));
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(h.app.world_mut(), actor, 1822, Some(target), false)
        {
            Ok(()) => {}
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NotEnoughPower(_),
            ) => {}
            Err(reason) => panic!("ordinary native Rake failed: {reason:?}"),
        }
        h.advance(1);
        if crate::auras::has_aura(h.app.world(), target, 58180) {
            return;
        }
    }
    panic!("explicit TDB48484 did not apply58180 after native Rake hits");
}

#[test]
#[ignore = "native private DB: generated391875 has authored100ms ICD and remains eligible"]
fn generic_proc_definition_preserves_generated_frantic_momentum() {
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor) = fixture(391875, false);
    assert!(!crate::auras::has_aura(h.app.world(), actor, 391876));
    cast_tigers_fury(&mut h, actor);
    assert!(crate::auras::has_aura(h.app.world(), actor, 391876));
}
