use super::*;

fn cast_native(run: &mut Run, spell: u32, target: Entity) {
    super::super::literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        super::super::literal_rotation::execution::Execution::Spell(
            spell,
            target,
            Some(run.targets[0]),
        ),
    )
    .expect("normal native cast");
    super::super::preparation::finish_pending_setup(run).unwrap();
}

fn cooldown_deadline(run: &Run, spell: u32) -> f64 {
    let world = run.app.world();
    let bindings = bindings();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(world, run.actor, &bindings, encounter, 1);
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap();
    crate::spell_cast::now(world) + facts.cooldown_for_spell(spell, "remains").unwrap()
}

#[test]
#[ignore = "requires exact Fury72 player profile and native selected Frenzy"]
fn fury_frenzy_uses_native_points_once_per_final_rampage_cast() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    let native_amount = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 335077)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(native_amount, 2.0);
    for expected_stacks in 1..=2 {
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            PowerType::Rage,
            1300.0,
        )
        .unwrap();
        cast_native(&mut run, 184367, target);
        let frenzy = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 335082)
            .expect("final Rampage cast grants native Frenzy");
        assert_eq!(
            frenzy.stacks, expected_stacks,
            "one stack per final Rampage attack, not per hit"
        );
        assert_eq!(
            frenzy
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount,
            native_amount
        );
    }
}

#[test]
#[ignore = "requires exact Fury72 player profile and native selected Anger Management"]
fn fury_anger_management_reduces_recklessness_from_actual_rampage_cost() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    let actor = run.actor;
    cast_native(&mut run, 1719, actor);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Rage, 1300.0)
        .unwrap();
    let before = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    let deadline = cooldown_deadline(&run, 1719);
    let target = run.targets[0];
    cast_native(&mut run, 184367, target);
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    let native_divisor = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Rage)
        .unwrap()
        .display_modifier as f64;
    let expected = f64::from(before - after) / native_divisor / 20.0;
    let reduction = deadline - cooldown_deadline(&run, 1719);
    assert!(expected > 0.0);
    assert!(
        (reduction - expected).abs() < 0.01,
        "actual spent rage must reduce native Recklessness: {reduction}, expected {expected}"
    );
    let paid_deadline = cooldown_deadline(&run, 1719);
    let hits = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(184707)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count()
    };
    let before_hits = hits(&run);
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 184367, Some(target), 1);
    super::super::preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        hits(&run) > before_hits,
        "free background Rampage must actually hit"
    );
    assert!(
        (cooldown_deadline(&run, 1719) - paid_deadline).abs() < 0.01,
        "a free background Rampage must not reduce cooldowns without a payment"
    );
}

#[test]
#[ignore = "requires exact Fury72 player profile and native selected Unhinged"]
fn fury_unhinged_bladestorm_ticks_execute_actual_bloodthirst_variant() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 1).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    cast_native(&mut run, 1719, actor);
    cast_native(&mut run, 446035, actor);
    for _ in 0..100 {
        run.app.update();
    }
    let combat = super::super::report::combat(run.app.world(), &run.damage_targets, 5.0);
    assert!(
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["spell_id"] == 335096 && action["hits"].as_u64().unwrap_or(0) > 0),
        "selected Unhinged must execute native Bloodbath during Recklessness: {combat}"
    );
}
