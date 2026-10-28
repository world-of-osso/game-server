//! Real Envenom applies the selected two-piece damage to native attacks.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::{components::PowerType, protocol::CombatLogKind};

fn damage(run: &mut Run, spell: Option<u32>, periodic: bool, start: usize) -> i32 {
    for _ in 0..160 {
        run.app.update();
        if let Some(event) = run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == spell
                    && event.kind == CombatLogKind::Damage
                    && event.periodic == periodic
                    && !event.crit
            })
        {
            return event.amount;
        }
    }
    panic!("actual attack {spell:?} did not deal noncritical damage");
}

fn sample(with_tier: bool) -> [i32; 3] {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1296590));
    if !with_tier {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1296590,
        );
    }
    crate::spell_cast::powers::change_power(
        run.app.world_mut(),
        actor,
        PowerType::ComboPoints,
        7.0,
    )
    .unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(32645, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 32645));
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 0.0;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        5374,
        Some(target),
        1
    ));
    let direct = damage(&mut run, Some(5374), false, start);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        385627,
        Some(target),
        1
    ));
    let periodic = damage(&mut run, Some(385627), true, start);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::combat::start_attack_now(
        run.app.world_mut(),
        actor,
        crate::combat::AutoAttack::new(target),
    );
    let white = damage(&mut run, None, false, start);
    [direct, periodic, white]
}

#[test]
#[ignore = "requires native259 profile and MID2 set"]
fn assassination_envenom_two_piece_scales_real_direct_periodic_and_auto_damage() {
    let baseline = sample(false);
    let tier = sample(true);
    for (plain, selected) in baseline.into_iter().zip(tier) {
        assert!(
            (f64::from(selected) / f64::from(plain) - 1.03).abs() < 0.02,
            "real Envenom must grant the native two-piece bonus: {plain} -> {selected}"
        );
    }
}
