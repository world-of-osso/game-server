//! Forlorn85: real1.5s cast + source3s missile delay before the source wild soul.
use super::*;

const SOUL_DISPLAYS: &[u32] = &[
    81277, 88427, 88428, 88429, 81278, 88430, 88431, 88432, 81205, 88433,
];

fn read_forlorn_souls(h: &mut Harness) -> Vec<(Entity, u32, f64)> {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{ModelDisplay, Npc};
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 143222 && summon.spell_id == 279222)
        .map(|(entity, _, summon)| {
            (
                entity,
                world.get::<ModelDisplay>(entity).unwrap().display_id,
                summon.expires_at.unwrap(),
            )
        })
        .collect()
}

#[test]
fn toyfx85_forlorn_real_learning_cast_flight_source_displays_recovery_and_soul_expiry() {
    use shared::components::{
        Health, ModelDisplay, Rotation, UnitFactionTemplate, UnitIsPet, UnitScale,
    };
    const ITEM: u32 = 163463;
    const SPELL: u32 = 279223;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("forlorn.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert((
        UnitScale(1.0),
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    assert!(
        read_forlorn_souls(&mut h).is_empty(),
        "source cast still pending"
    );
    h.advance(35); // Total2.1s allows message delivery; fixed3s travel is not over.
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert!(read_forlorn_souls(&mut h).is_empty());
    h.advance(30); // Total3.6s: still before1.5+3s, not radius3/speed3=1s.
    assert!(
        read_forlorn_souls(&mut h).is_empty(),
        "speed-as-delay must remain3s"
    );
    h.advance(25);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let souls = read_forlorn_souls(&mut h);
    assert_eq!(souls.len(), 1);
    let (first, first_display, first_expiry) = souls[0];
    assert!(SOUL_DISPLAYS.contains(&first_display));
    let first_position = *h.app.world().get::<Position>(first).unwrap();
    assert!(
        (first_position.x - origin.x - 3.0).abs() < 0.01,
        "source47/radius3: soul={first_position:?}, origin={origin:?}, caster={:?}, facing={:?}",
        h.app.world().get::<Position>(caster),
        h.app.world().get::<Rotation>(caster)
    );
    assert!((first_position.z - origin.z).abs() < 0.01);
    assert_eq!(
        h.app.world().get::<UnitFactionTemplate>(first),
        Some(&UnitFactionTemplate(35))
    );
    assert!(
        !h.app
            .world()
            .get::<UnitIsPet>(first)
            .is_some_and(|pet| pet.0)
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(first)
            .is_none()
    );
    let remaining = first_expiry - crate::auras::aura_now(h.app.world());
    assert!((59.0..=60.0).contains(&remaining), "source60s: {remaining}");
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|cooldown| cooldown.spell_id == SPELL && !cooldown.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, 30_000);
    assert_eq!(
        activate(&mut h, ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );

    h.advance(620); // Source30s recovery elapses while first60s actor remains live.
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x += 10.0;
    h.advance(20);
    assert_eq!(h.app.world().get::<Position>(first), Some(&first_position));
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(100);
    let souls = read_forlorn_souls(&mut h);
    assert_eq!(souls.len(), 2, "wild slot0 does not replace the first soul");
    let (second, second_display, second_expiry) =
        *souls.iter().find(|soul| soul.0 != first).unwrap();
    assert!(SOUL_DISPLAYS.contains(&second_display));
    let first_remaining = first_expiry - crate::auras::aura_now(h.app.world());
    h.advance((first_remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get_entity(first).is_err());
    assert!(h.app.world().get_entity(second).is_ok());
    let second_remaining = second_expiry - crate::auras::aura_now(h.app.world());
    h.advance((second_remaining / 0.05).ceil() as usize + 2);
    assert!(read_forlorn_souls(&mut h).is_empty());
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}
