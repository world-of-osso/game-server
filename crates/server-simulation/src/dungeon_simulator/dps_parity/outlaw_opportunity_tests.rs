use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_background_sinister_extra_cannot_earn_fang_strike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296589));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 1301405));
    for _ in 0..32 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            197834,
            Some(run.targets[0]),
            1,
        );
        run.app.update();
    }
    let hits = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(197834)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hits >= 24,
        "actual native extra strikes must reach their hostile recipient: {hits}"
    );
    // SimC76f8118 sc_rogue.cpp3597/5991 are the only tier trigger calls;
    // sinister_strike_extra_attack_t5944–5964 does not trigger the four-piece.
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1301405),
        "background Sinister extra strikes cannot produce a Fang Strike window"
    );
}

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_sinister_strike_earns_opportunity_and_delayed_extra_attack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        {
            let mut pools = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                193315,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        if crate::auras::has_aura(run.app.world(), run.actor, 195627) {
            break;
        }
    }
    // SimC76f8118 sc_rogue.cpp8255–8280: landed Sinister Strike rolls
    // Opportunity and dispatches its independent extra attack after300ms.
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 195627),
        "ordinary native Sinister Strikes must earn Opportunity"
    );
    for _ in 0..10 {
        run.app.update();
    }
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
                && event.spell_id == Some(197834)
                && event.amount > 0),
        "earned Opportunity must launch the actual native extra strike on the source target"
    );
}
