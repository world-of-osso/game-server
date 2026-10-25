//! Physical learning/UseToy for Ruthers;73 alone must not fabricate other toy actions.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::{Health, ModelDisplay, Npc, UnitSummonedBy};

const RUTHERS_TOY: u32 = 82467;
const RUTHERS_SPELL: u32 = 120108;
const RUTHERS_ENTRY: u32 = 61667;
const RECOVERY_MS: u32 = 1_800_000;

fn fixture(path: &std::path::Path) -> (Harness, u64) {
    let (mut h, character) = setup(path);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .unwrap(),
    );
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    learn_effect_toy(&mut h, character, RUTHERS_TOY);
    (h, character)
}

fn ruthers(h: &mut Harness) -> Vec<Entity> {
    let owner = h.caster;
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| {
            npc.template_id == RUTHERS_ENTRY
                && summon.owner == owner
                && summon.spell_id == RUTHERS_SPELL
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

#[test]
fn toy_coverage_target73_ruthers_real_use_actor_expiry_and_natural_recovery_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = fixture(&dir.path().join("ruthers.redb"));
    let owner = h.caster;
    let health = *h.app.world().get::<Health>(owner).unwrap();
    let model = h.app.world().get::<ModelDisplay>(owner).copied();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, RUTHERS_TOY)[0].error, None);
    let actors = ruthers(&mut h);
    assert_eq!(actors.len(), 1);
    let actor = actors[0];
    let world = h.app.world();
    let source = &world.resource::<crate::world_data::WorldData>().templates[&RUTHERS_ENTRY];
    let display = world.get::<ModelDisplay>(actor).unwrap().display_id;
    assert!(
        source
            .models
            .iter()
            .any(|model| model.display_id == display)
    );
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        owner.to_bits()
    );
    assert_eq!(
        world.get::<crate::maps::InMap>(actor),
        world.get::<crate::maps::InMap>(owner)
    );
    let actual = *world.get::<Position>(actor).unwrap();
    let distance = Vec2::new(actual.x - position.x, actual.z - position.z).length();
    assert!(
        (distance - 5.0).abs() < 0.001,
        "source caster-radius5 actual{actual:?}"
    );
    assert!(
        !world
            .get::<SummonedCreature>(actor)
            .unwrap()
            .properties
            .is_guardian()
    );
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == RUTHERS_SPELL && cd.duration_ms == RECOVERY_MS && !cd.is_gcd)
    );
    h.advance(580); // Native50ms ticks: retain actor before authored30s expiry.
    assert!(h.app.world().get_entity(actor).is_ok());
    h.advance(40);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(
        activate(&mut h, RUTHERS_TOY)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(35_500); // Naturally cross authored30-minute item recovery.
    assert_eq!(activate(&mut h, RUTHERS_TOY)[0].error, None);
    let next = ruthers(&mut h);
    assert_eq!(next.len(), 1);
    assert_ne!(next[0], actor);
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|go| go.spell_id == RUTHERS_SPELL)
            .count(),
        2
    );
    assert_eq!(h.app.world().get::<Health>(owner), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(owner).copied(), model);
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
}

#[test]
fn toy_coverage_target73_ruthers_missing_collision_refuses_without_actor_or_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = fixture(&dir.path().join("ruthers-collision.redb"));
    let collision = h
        .app
        .world_mut()
        .remove_resource::<crate::line_of_sight::LineOfSight>()
        .expect("real collision fixture authority");
    let result = activate(&mut h, RUTHERS_TOY);
    assert!(
        result.iter().any(|result| result
            .error
            .as_deref()
            .is_some_and(|reason| reason.contains("target73 requires collision data"))),
        "{result:?}"
    );
    assert!(ruthers(&mut h).is_empty());
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == RUTHERS_SPELL));
    println!(
        "Ruthers rejected cast cooldown receipts: {:?}",
        h.inbox.cooldowns
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == RUTHERS_SPELL && !cd.is_gcd && cd.duration_ms > 0)
    );
    // A cast-start GCD is not the thirty-minute item recovery. Wait it out,
    // restore actual collision authority and prove a real retry is not OnCooldown.
    h.advance(40);
    h.app.insert_resource(collision);
    assert_eq!(activate(&mut h, RUTHERS_TOY)[0].error, None);
    assert_eq!(ruthers(&mut h).len(), 1);
}
