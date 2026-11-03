//! Follower upkeep must not spend recovery mana on full-health healing.
use shared::components::{ActiveSpec, Health, Mana, Position};
use shared::protocol::CombatLogKind;

fn check_full_health_upkeep(spec: u32, excluded_heals: &[u32], check_mark: bool) {
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
        [66, spec, 262, 63, 253],
    )
    .unwrap();
    let healer = *party
        .members
        .iter()
        .find(|&&unit| {
            app.world()
                .get::<ActiveSpec>(unit)
                .is_some_and(|active| active.0 == spec)
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
        if check_mark {
            crate::auras::remove_auras(
                app.world_mut(),
                unit,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 1126,
            );
        }
    }
    if spec == 264 {
        crate::auras::remove_auras(
            app.world_mut(),
            healer,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 52127,
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
        "spec {spec} failed to recover: {mana:?}"
    );
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(healer.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .collect();
    assert!(
        !casts.iter().any(|event| event
            .spell_id
            .is_some_and(|spell| excluded_heals.contains(&spell))),
        "spec {spec} cast healing upkeep: {casts:?}"
    );
    if spec == 264 {
        assert!(
            casts.iter().any(|event| event.spell_id == Some(52127)),
            "missing Water Shield must be recast as a maintained stat buff"
        );
    }
    if check_mark {
        assert!(
            events
                .iter()
                .any(|event| event.spell_id == Some(1126)
                    && event.kind == CombatLogKind::CastSuccess),
            "missing Mark of the Wild must be recast"
        );
    }
}

#[test]
fn restoration_druid_recovers_mana_without_full_health_hots_and_recasts_mark() {
    check_full_health_upkeep(105, &[774, 33763, 8936], true);
}

#[test]
fn other_healers_do_not_cast_healing_upkeep_on_full_health_party() {
    // Riptide, Renew, Atonement producers and Echo respectively.
    for (spec, heals) in [
        (264, &[61295, 8004, 77472][..]),
        (257, &[139, 2061, 2060][..]),
        (256, &[17, 194509, 2061][..]),
        (1468, &[364343, 361469, 355936][..]),
    ] {
        check_full_health_upkeep(spec, heals, false);
    }
}
