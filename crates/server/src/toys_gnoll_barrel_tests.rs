//! Gnoll Barrel81: source paired wild actors, not a fabricated combat dummy AI.
use super::*;

#[test]
fn toyfx81_gnoll_barrel_real_learning_source_area_cursor_pair_and_expiry() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitFactionTemplate, UnitIsPet};
    const ITEM: u32 = 163201;
    const SPELL: u32 = 279063;
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("gnoll.redb"));
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, ITEM);
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let position = *h.app.world().get::<Position>(caster).unwrap();
    let error = activate_at(
        &mut h,
        ITEM,
        Some([position.x + 2.0, position.y, position.z]),
    )[0]
    .error
    .clone()
    .expect("Northshire is outside source area group5759");
    assert!(error.contains("5759"), "{error}");
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == SPELL)
    );
    h.inbox.failed.clear();

    // Source KulTiras_30_22 MCNK is actual area9042 (group5759), not relabeled terrain.
    place_toy_caster_on_source_map_area(&mut h, 1643, 1424104, [4883.3335, 716.6663], 9042);
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    let point = [origin.x + 2.0, origin.y, origin.z];
    assert_eq!(activate_at(&mut h, ITEM, Some(point))[0].error, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPELL));
    assert!(h.inbox.cooldowns.iter().any(|cooldown| {
        cooldown.spell_id == SPELL && !cooldown.is_gcd && cooldown.duration_ms == 1_800_000
    }));
    let world = h.app.world_mut();
    let mut actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, _, summon)| summon.spell_id == SPELL)
        .map(|(entity, npc, summon)| (npc.template_id, entity, summon.expires_at.unwrap()))
        .collect();
    actors.sort_by_key(|actor| actor.0);
    assert_eq!(
        actors.iter().map(|actor| actor.0).collect::<Vec<_>>(),
        [143119, 143120]
    );
    let expected_displays = [38854, 38857];
    for ((_, actor, expires_at), expected_display) in actors.iter().zip(expected_displays) {
        assert_eq!(
            world.get::<ModelDisplay>(*actor).unwrap().display_id,
            expected_display
        );
        assert_eq!(
            world.get::<UnitFactionTemplate>(*actor),
            Some(&UnitFactionTemplate(35))
        );
        assert!(!world.get::<UnitIsPet>(*actor).is_some_and(|pet| pet.0));
        assert!(
            world
                .get::<crate::creature_ai::FollowOwner>(*actor)
                .is_none()
        );
        assert_eq!(
            world.get::<crate::maps::InMap>(*actor),
            world.get::<crate::maps::InMap>(caster)
        );
        let position = *world.get::<Position>(*actor).unwrap();
        assert!((position.x - point[0]).abs() < 0.01);
        assert!((position.z - point[2]).abs() < 0.01);
        let remaining = expires_at - crate::auras::aura_now(world);
        assert!(
            (299.0..=300.0).contains(&remaining),
            "source300s: {remaining}"
        );
    }
    let positions: Vec<_> = actors
        .iter()
        .map(|(_, actor, _)| *world.get::<Position>(*actor).unwrap())
        .collect();
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x += 20.0;
    h.advance(10);
    for ((_, actor, _), position) in actors.iter().zip(&positions) {
        assert_eq!(h.app.world().get::<Position>(*actor), Some(position));
    }
    let remaining = actors[0].2 - crate::auras::aura_now(h.app.world());
    h.advance((remaining / 0.05).ceil() as usize + 2);
    for (_, actor, _) in actors {
        assert!(h.app.world().get_entity(actor).is_err());
    }
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        activate_at(&mut h, ITEM, Some(point))[0].error.as_deref(),
        Some("OnCooldown")
    );
}
