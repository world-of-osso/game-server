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
    // Net Rage also includes gains during Rampage's native completion window.
    // Read the actual native payment input, not the later bank balance.
    let rampage = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(184367)
        .unwrap();
    let paid = crate::spell_cast::applicable_costs(run.app.world(), actor, rampage)
        .unwrap()
        .iter()
        .filter(|(pool, _, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::Rage))
        .map(|(_, amount, _)| *amount)
        .sum::<f32>();
    assert_eq!(paid, 800.0, "actual native Rampage costs80 displayed Rage");
    let deadline = cooldown_deadline(&run, 1719);
    let target = run.targets[0];
    cast_native(&mut run, 184367, target);
    let native_divisor = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Rage)
        .unwrap()
        .display_modifier as f64;
    let expected = f64::from(paid) / native_divisor / 20.0;
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
#[ignore = "requires exact Fury72 player profile and native Bladestorm children"]
fn fury_bladestorm_native_ticks_execute_one_offhand_area_cast() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = super::super::prepare_measurement(72, 3).unwrap();
    super::super::engage(&mut run);
    let actor = run.actor;
    cast_native(&mut run, 446035, actor);
    run.app
        .world_mut()
        .resource_mut::<Time<Fixed>>()
        .set_timestep_seconds(3.0);
    for _ in 0..100 {
        run.app.update();
    }
    let combat = super::super::report::combat(run.app.world(), &run.damage_targets, 5.0);
    let attempts = |spell: u32| {
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["spell_id"] == spell)
            .map(|action| {
                ["hits", "misses", "dodges", "parries"]
                    .iter()
                    .map(|field| action[*field].as_u64().unwrap())
                    .sum::<u64>()
            })
            .sum::<u64>()
    };
    assert!(attempts(50622) >= 3, "native main-hand area must execute");
    // SimC76f8118 sc_warrior.cpp3249–3254/3335–3340: one native off-hand
    // area follows each main-hand tick, not one area per main-hand recipient.
    assert_eq!(
        attempts(95738),
        attempts(50622),
        "each native Fury Bladestorm tick must execute one off-hand area: {combat}"
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
    // Process several due aura ticks in one fixed update. Each must keep its
    // own ordinal, rather than reading the scheduler's final paid count.
    run.app
        .world_mut()
        .resource_mut::<Time<Fixed>>()
        .set_timestep_seconds(3.0);
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
    let hits = |spell: u32| {
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["spell_id"] == spell)
            .map(|action| action["hits"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert_eq!(
        hits(335096),
        hits(50622) / 2,
        "one Bloodbath per odd Bladestorm tick, not every tick or every impact"
    );
}
