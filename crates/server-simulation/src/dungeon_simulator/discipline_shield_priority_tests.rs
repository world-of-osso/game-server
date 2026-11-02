//! Retail-inferred ordinary Shield priority for a real Discipline follower.

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_does_not_refresh_an_active_ordinary_shield() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    use std::collections::HashSet;

    let _seed = crate::random::seed_scope(1);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = run.healer;
    run.app
        .insert_resource(server_follower_apl::registry::FixedLengthFight { duration: 120.0 });
    super::healer_simc_replay::engage(&mut run);
    super::healer_simc_replay::finish_precombat(&mut run.app, &[run.tank, healer]).unwrap();
    // Isolate the Shield and damage choices so the live follower repeatedly
    // reaches this priority instead of spending the fixture on unrelated cooldowns.
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            17, 585, 390691,
        ]));
    for member in run.slots {
        crate::auras::remove_auras(
            run.app.world_mut(),
            member,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 17 && aura.caster == healer,
        );
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: healer,
                target: member,
                spell_id: 194384,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    let mut shielded: HashSet<_> = run
        .slots
        .iter()
        .copied()
        .filter(|unit| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(*unit)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 17 && aura.caster == healer)
                })
        })
        .map(|unit| unit.to_bits())
        .collect();
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..400 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    let mut casts = 0;
    for event in events
        .iter()
        .filter(|event| event.source == Some(healer.to_bits()) && event.spell_id == Some(17))
    {
        let target = event.target.expect("Shield event has a target");
        match event.kind {
            CombatLogKind::CastSuccess => {
                casts += 1;
                assert!(
                    !shielded.contains(&target),
                    "Shield was recast while active: {event:?}"
                );
            }
            CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed => {
                shielded.insert(target);
            }
            CombatLogKind::AuraRemoved => {
                shielded.remove(&target);
            }
            _ => {}
        }
    }
    assert!(
        casts > 0,
        "the follower must still cast ordinary Shield; healer casts: {:?}",
        events
            .iter()
            .filter(|event| event.source == Some(healer.to_bits())
                && event.kind == CombatLogKind::CastSuccess)
            .collect::<Vec<_>>()
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(585)),
        "the follower must continue to cast Smite"
    );
}
