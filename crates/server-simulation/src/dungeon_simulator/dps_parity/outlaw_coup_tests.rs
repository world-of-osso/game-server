use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn read_full_escalation(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 441786 && aura.caster == run.actor)
        .is_some_and(|aura| aura.max_stacks > 0 && aura.stacks == aura.max_stacks)
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_coup_grants_ruthlessness_and_bonus_cdr_for_paid_and_fang_finishers() {
    for fang in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(260, 1).unwrap();
        engage(&mut run);
        for _ in 0..200 {
            if read_full_escalation(&run)
                && (!fang || crate::auras::has_aura(run.app.world(), run.actor, 1301405))
            {
                break;
            }
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
                literal_rotation::execution::Execution::Spell(
                    193315,
                    run.targets[0],
                    Some(run.targets[0]),
                ),
            )
            .unwrap();
            preparation::finish_pending_setup(&mut run).unwrap();
        }
        assert!(read_full_escalation(&run), "Coup must be earned");
        if fang {
            assert!(crate::auras::has_aura(run.app.world(), run.actor, 1301405));
        } else {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 1301405 && aura.caster == run.actor,
            );
        }
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = if fang { 0.0 } else { energy.max };
            powers.get_mut(PowerType::ComboPoints).unwrap().current = if fang { 0.0 } else { 6.0 };
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(2983, run.actor, Some(run.targets[0])),
        )
        .unwrap();
        let before = sprint_cooldown_ms(&run);
        let per_point =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 79096, 0)
                / 10.0;
        let bonus =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 441423, 2);
        assert_eq!(bonus, 5.0, "native Coup effective point bonus");
        let expected = ((if fang { 7.0 } else { 6.0 }) + bonus) * per_point * 1000.0;
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                441776,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        let reduction = before.saturating_sub(sprint_cooldown_ms(&run));
        assert!(
            (reduction as f32 - expected).abs() <= 2.0,
            "earned Coup refunds its bonus once: fang={fang}, reduction={reduction}, expected={expected}"
        );
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let grants = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.actor.to_bits())
                    && event.spell_id == Some(1258235)
                    && event.kind == CombatLogKind::Energize
            })
            .count();
        assert!(
            (2..=3).contains(&grants),
            "Coup must award Ruthlessness once with its native bonus points; fang={fang}, grants={grants}"
        );
    }
}

fn sprint_cooldown_ms(run: &Run) -> u64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(2983)
        .unwrap();
    world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap()
        .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_earned_coup_executes_three_native_children_and_consumes_escalation() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 441776)
            .is_err(),
        "selected Coup must not be admitted before escalation is earned"
    );
    for _ in 0..96 {
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
            literal_rotation::execution::Execution::Spell(
                193315,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        if read_full_escalation(&run) {
            break;
        }
    }
    assert!(
        read_full_escalation(&run),
        "real Sinister/Unseen executions must earn maximum Escalating Blade"
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 2098)
            .is_err(),
        "earned Coup must replace Dispatch until it is consumed"
    );
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        // Below native Gravedigger's threshold; Coup's bonus points qualify it.
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 1.0;
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    // sc_rogue.cpp6863–6904: actual earned Coup is an ordinary native
    // finisher, not a forged triggered root or an injected learned-spell grant.
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(441776, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [462140, 462239, 462240] {
        let attempts = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .count();
        assert_eq!(
            attempts, 1,
            "earned native Coup must dispatch exactly one{spell} child"
        );
    }
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && [Some(462140), Some(462239), Some(462240)].contains(&event.spell_id)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1265918)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))),
        "Coup's bonus effective points must earn its own Scoundrel Strike"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 441786),
        "successful native Coup must consume its earned escalation"
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 2098)
            .is_ok()
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 441776)
            .is_err()
    );
}
