use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical 266 inputs; real Inquisitor Mind Sear cast and channel"]
fn demonology_inquisitor_mind_sear_ticks_after_hasted_precast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 3).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 1276283, Some(owner), true)
        .unwrap();
    let inquisitor = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 1276283)
        .map(|(actor, _)| actor)
        .expect("real Antoran Inquisitor");
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1280457)
        .unwrap();
    let duration = spell.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    let cast_ms = crate::spell_cast::cast_time_ms(run.app.world(), inquisitor, spell);
    assert!(
        cast_ms > 0 && cast_ms < duration,
        "native duration must receive owner haste"
    );
    for _ in 0..(cast_ms as usize / 50).saturating_sub(10) {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(inquisitor.to_bits())
                && event.spell_id == Some(1280460)),
        "Mind Sear cannot tick before its hasted cast completes"
    );
    for _ in 0..20 {
        run.app.update();
    }
    let hits = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            hits.iter()
                .any(|event| event.source == Some(inquisitor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1280460)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "the completed cast begins its channel with a tick against each target"
        );
    }
}

#[test]
#[ignore = "requires canonical 266 inputs; overlapping Inquisitor cast and channel"]
fn demonology_inquisitor_recasts_mind_sear_during_channel() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let owner = run.actor;
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .haste_pct = 150.0;
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, 1276283, Some(owner), true)
        .unwrap();
    let inquisitor = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 1276283)
        .map(|(actor, _)| actor)
        .expect("real Antoran Inquisitor");
    for _ in 0..190 {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(owner)
            .unwrap()
            .haste_pct = 150.0;
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(inquisitor).is_ok(),
        "native ten-second summon remains alive"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(inquisitor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1280457)
                && event.kind == CombatLogKind::AuraRefreshed
        }),
        "real Inquisitor must refresh its channel during the second execute"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(inquisitor)
            .is_some_and(|state| state.spell_id == 1280457
                && state.cast_type == shared::casting::CastType::Channel),
        "the original channel must still be active after the repeat"
    );
}
