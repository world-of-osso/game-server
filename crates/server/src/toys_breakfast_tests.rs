//! Source135/149: authentic scenery tables and ordinary five-minute lifetimes.
use super::*;

struct SourceTableCase {
    item: u32,
    spell: u32,
    entry: u32,
    display: u32,
    scale: f32,
    front_distance: f32,
    recovery_ms: u32,
}

#[test]
fn toyfx135_goldtusk_real_learning_source_table_scale_expiry_and_recovery() {
    assert_source_table_lifecycle(SourceTableCase {
        item: 166703,
        spell: 289533,
        entry: 149876,
        display: 90570,
        scale: 0.5,
        front_distance: 3.0,
        recovery_ms: 600_000,
    });
}

#[test]
fn toyfx149_tea_set_real_learning_source_table_scale_expiry_and_recovery() {
    assert_source_table_lifecycle(SourceTableCase {
        item: 166808,
        spell: 290483,
        entry: 149865,
        display: 90552,
        scale: 1.0,
        front_distance: 1.5,
        recovery_ms: 900_000,
    });
}

fn assert_source_table_lifecycle(case: SourceTableCase) {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{
        Health, ModelDisplay, Npc, Rotation, UnitFactionTemplate, UnitIsPet, UnitScale,
        UnitSummonedBy,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("breakfast.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    let map = *h.app.world().get::<crate::maps::InMap>(caster).unwrap();
    learn_effect_toy(&mut h, character, case.item);
    assert_eq!(activate(&mut h, case.item)[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == case.spell));
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == case.entry && summon.spell_id == case.spell)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(actors.len(), 1);
    let (actor, expiry) = actors[0];
    assert_eq!(
        world.get::<ModelDisplay>(actor).unwrap().display_id,
        case.display
    );
    assert_eq!(world.get::<UnitScale>(actor).unwrap().0, case.scale);
    assert_eq!(world.get::<UnitFactionTemplate>(actor).unwrap().0, 35);
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        caster.to_bits()
    );
    assert_eq!(world.get::<crate::maps::InMap>(actor), Some(&map));
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let placed = *world.get::<Position>(actor).unwrap();
    assert!(
        (placed.x - origin.x - case.front_distance).abs() < 0.01
            && (placed.z - origin.z).abs() < 0.01,
        "authored47 source front placement: {placed:?}"
    );
    let remaining = expiry - world.resource::<Time>().elapsed_secs_f64();
    assert!((299.0..=300.0).contains(&remaining));
    assert_eq!(
        h.inbox
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.spell_id == case.spell && !cooldown.is_gcd)
            .unwrap()
            .duration_ms,
        case.recovery_ms
    );
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x -= 5.0;
    h.advance(20);
    assert_eq!(h.app.world().get::<Position>(actor), Some(&placed));
    let remaining = expiry - h.app.world().resource::<Time>().elapsed_secs_f64();
    h.advance((remaining / 0.05).floor() as usize - 2);
    assert!(h.app.world().get_entity(actor).is_ok());
    h.advance(6);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        activate(&mut h, case.item)[0].error.as_deref(),
        Some("OnCooldown"),
        "five-minute actor expiry retains the source item recovery"
    );
}
