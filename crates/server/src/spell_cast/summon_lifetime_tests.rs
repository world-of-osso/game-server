//! Wild temporary summons retain their own lifetime after summoner death.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

#[test]
fn encounter_coverage_wild_summon_outlives_owner_but_keeps_authored_expiry() {
    let mut h = harness(Vec::new(), &[30707], 9).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let properties = super::summon::load_summon_properties(&conn).unwrap().0[&64];
    assert_eq!(
        (properties.control, properties.title, properties.flags),
        (0, 0, 0)
    );
    let actor = spawn_npc(&mut h.app, 3.0);
    let now = crate::auras::aura_now(h.app.world());
    h.app
        .world_mut()
        .entity_mut(actor)
        .insert(super::summon::SummonedCreature {
            owner: h.caster,
            spell_id: 30707,
            properties_id: 64,
            properties,
            display_scale: 1.0,
            expires_at: Some(now + 1.0),
        });
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(2);
    assert!(
        h.app.world().get_entity(actor).is_ok(),
        "wild summon retains independent lifetime after owner death"
    );
    h.advance(30);
    assert!(
        h.app.world().get_entity(actor).is_err(),
        "owner death must not disable authored expiry"
    );
}
