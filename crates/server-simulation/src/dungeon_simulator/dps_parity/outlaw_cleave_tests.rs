use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_blade_flurry_transfers_native_hits_only_to_secondary_targets() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 3).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(13877, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 13877));
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(193315, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(193315)
                && event.amount > 0),
        "native primary hit must land"
    );
    let primary_amount = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(193315)
        })
        .unwrap()
        .amount as f32;
    let fraction = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 13877 && aura.caster == run.actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount
        / 100.0;
    let expected_copy = primary_amount * fraction;
    let copies: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(22482)
                && event.amount > 0
        })
        .collect();
    for &secondary in &run.targets[1..] {
        assert!(
            copies
                .iter()
                .any(|event| event.target == Some(secondary.to_bits())),
            "native Blade Flurry must transfer landed hits to secondary{secondary:?}"
        );
    }
    for &secondary in &run.targets[1..] {
        assert!(
            copies.iter().any(|event| {
                event.target == Some(secondary.to_bits())
                    && (event.amount as f32 - expected_copy).abs() <= 2.0
            }),
            "secondary{secondary:?} must receive the native fraction of this same-armor source hit ({expected_copy}), without doubling caster bonuses, critical damage or armor"
        );
    }
    assert!(
        !copies
            .iter()
            .any(|event| event.target == Some(run.targets[0].to_bits())),
        "transfer must not duplicate damage onto the primary target"
    );
}
