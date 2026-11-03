//! Follower upkeep must not spend recovery mana on full-health healing.
use shared::components::{ActiveSpec, Health, Mana, Position};
use shared::protocol::CombatLogKind;

#[test]
fn restoration_druid_recovers_mana_without_full_health_hots_and_recasts_mark() {
    let _seed = crate::random::seed_scope(2);
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_test_party(
        app.world_mut(),
        copy,
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        90,
        [66, 105, 262, 63, 253],
    )
    .unwrap();
    let healer = *party
        .members
        .iter()
        .find(|&&unit| {
            app.world()
                .get::<ActiveSpec>(unit)
                .is_some_and(|spec| spec.0 == 105)
        })
        .unwrap();
    // Let native class initialization finish before setting the recovery state.
    for _ in 0..100 {
        app.update();
    }
    for unit in std::iter::once(party.owner).chain(party.members.iter().copied()) {
        if let Some(mut health) = app.world_mut().get_mut::<Health>(unit) {
            health.current = health.max;
        }
        crate::auras::remove_auras(
            app.world_mut(),
            unit,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1126,
        );
    }
    {
        let mut mana = app.world_mut().get_mut::<Mana>(healer).unwrap();
        mana.current = mana.max * 0.79;
    }
    let cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    for _ in 0..1200 {
        app.update();
    }
    let mana = app.world().get::<Mana>(healer).unwrap();
    let events = &app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[cursor..];
    assert!(
        mana.current / mana.max >= 0.81,
        "healer failed to recover: {mana:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && event.spell_id == Some(774)
                && event.kind == CombatLogKind::CastSuccess),
        "full-health upkeep must not cast Rejuvenation"
    );
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(1126) && event.kind == CombatLogKind::CastSuccess),
        "missing Mark of the Wild must be recast"
    );
}
