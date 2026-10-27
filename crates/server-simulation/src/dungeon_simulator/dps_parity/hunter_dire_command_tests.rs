use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_dire_command_actual_kills_spawn_authentic_beasts() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 378743));
    let probability = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        378743,
        0,
    )) / 100.0;
    assert!(probability > 0.0);
    let maximum_attempts =
        (1.0 / crate::random::accumulated_proc_constant(probability)).ceil() as usize;
    let mut query = run.app.world_mut().query::<(
        Entity,
        &crate::spell_cast::summon::SummonedCreature,
        &shared::components::Npc,
    )>();
    let mut beasts = std::collections::BTreeSet::new();
    let mut origins = std::collections::HashMap::new();
    let mut moved_toward_target = false;
    engage(&mut run);
    advance_combat_observed(&mut run, |run| {
        for (unit, summon, npc) in query.iter(run.app.world()) {
            if summon.owner == run.actor && summon.spell_id == 219199 {
                assert_eq!(
                    npc.template_id, 110063,
                    "Dire Command must retain native summon identity"
                );
                assert!(
                    summon.expires_at.is_some(),
                    "native Dire Beast must be finite"
                );
                let position = *run
                    .app
                    .world()
                    .get::<shared::components::Position>(unit)
                    .unwrap();
                let target = *run
                    .app
                    .world()
                    .get::<shared::components::Position>(run.targets[0])
                    .unwrap();
                let distance =
                    ((position.x - target.x).powi(2) + (position.z - target.z).powi(2)).sqrt();
                let original = *origins.entry(unit.to_bits()).or_insert(distance);
                moved_toward_target |= distance + 0.1 < original;
                beasts.insert(unit.to_bits());
            }
        }
    })
    .expect("ordinary native300s literal combat");
    let commands = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(34026)
                && event.kind == shared::protocol::CombatLogKind::CastSuccess
        })
        .count();
    assert!(
        commands >= maximum_attempts,
        "fixture must execute enough actual KCs to exceed the native PRD guarantee: commands={commands}, guarantee={maximum_attempts}"
    );
    // SimC76f8118 hunter6610–6613/8396: actual successful KC drives the
    // selected native chance's PRD, not an unearned manual summon cast.
    assert!(
        !beasts.is_empty(),
        "actual KC Dire Command must spawn native219199 actors: commands={commands}, beasts={}",
        beasts.len()
    );
    assert!(
        moved_toward_target,
        "a real summoned Beast outside melee range must move toward its target"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| beasts.contains(&source))
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id.is_none()
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::Damage
                        | shared::protocol::CombatLogKind::Miss(_)
                )
        ),
        "actual Dire Beasts must make native white attacks at the real primary"
    );
    assert!(
        events.iter().any(
            |event| event.source.is_some_and(|source| beasts.contains(&source))
                && event.spell_id == Some(1232922)
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::Damage
                        | shared::protocol::CombatLogKind::Miss(_)
                )
        ),
        "existing actual Dire Beasts must receive selected Wildspeaker commands"
    );
}
