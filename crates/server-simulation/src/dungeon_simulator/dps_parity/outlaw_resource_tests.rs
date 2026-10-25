use super::*;
use server::simc_apl::Facts;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn fund_finisher(run: &mut Run, points: f32) {
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.caster == run.actor && aura.spell_id == 1301405,
    );
    let mut powers = run
        .app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap();
    let energy = powers.get_mut(PowerType::Energy).unwrap();
    energy.current = energy.max;
    powers.get_mut(PowerType::ComboPoints).unwrap().current = points;
}

fn first_finisher_damage(spec: u32, spell: u32, points: f32) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(spec, 1).unwrap();
    engage(&mut run);
    fund_finisher(&mut run, points);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hit = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
        })
        .unwrap();
    assert!(
        !hit.crit,
        "the point-value control excludes critical amplification"
    );
    assert!(hit.amount > 0);
    f64::from(hit.amount + hit.blocked + hit.absorbed + hit.resisted)
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_between_the_eyes_values_actual_combo_points() {
    let one = first_finisher_damage(260, 315341, 1.0);
    let five = first_finisher_damage(260, 315341, 5.0);
    let ratio = five / one;
    assert!(
        (4.5..5.5).contains(&ratio),
        "five native points must scale the one-point payload, not repeat it: {one}/{five}, ratio={ratio}"
    );
}

#[test]
#[ignore = "requires private native259/260 player profiles and traits"]
fn rogue_envenom_and_dispatch_retain_paid_point_scaling() {
    for (spec, spell) in [(259, 32645), (260, 2098)] {
        let one = first_finisher_damage(spec, spell, 1.0);
        let five = first_finisher_damage(spec, spell, 5.0);
        let ratio = five / one;
        assert!(
            (4.5..5.5).contains(&ratio),
            "native finisher{spell} must retain five-point scaling: {one}/{five}, ratio={ratio}"
        );
    }
}

fn between_the_eyes_remaining(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("between_the_eyes".into(), 315341)].into(),
        ..default()
    };
    crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.between_the_eyes.remains")
    .unwrap()
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_finisher_reduces_running_cooldowns_by_effective_points() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    fund_finisher(&mut run, 4.0);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(315341, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let remaining = between_the_eyes_remaining(&run);
    assert!(remaining > 10.0, "native cooldown must be running");
    fund_finisher(&mut run, 4.0);
    let before = crate::spell_cast::history_now(run.app.world());
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(2098, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let elapsed = crate::spell_cast::history_now(run.app.world()) - before;
    let reduction = remaining - elapsed - between_the_eyes_remaining(&run);
    assert!(
        (3.9..4.1).contains(&reduction),
        "four effective points must refund four native seconds beyond elapsed time: {reduction}"
    );
}
