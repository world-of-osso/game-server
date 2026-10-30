//! Native Vengeful Wrath target-health boundaries on real Hammer damage.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn hammer(health_fraction: f32, selected: bool) -> (f32, f32) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(66, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1241958));
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 1241958, 0);
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1241958,
        );
    }
    let mut health = run
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap();
    health.current = health.max * health_fraction;
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    let _hit_seed = crate::random::seed_scope(12345);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        1241413,
        Some(target),
        0
    ));
    for _ in 0..40 {
        run.app.update();
    }
    let event = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(1241413)
                && event.kind == CombatLogKind::Damage
        })
        .expect("real Hammer of Wrath hit");
    (event.amount as f32, percent)
}

#[test]
#[ignore = "native66 reference inputs and private world database"]
fn vengeful_wrath_scales_real_hammer_between_full_and_execute_health() {
    let (full, percent) = hammer(1.0, true);
    let (halfway, _) = hammer(0.6, true);
    let (execute, _) = hammer(0.2, true);
    let (untalented, _) = hammer(0.2, false);
    assert!(percent > 0.0);
    assert!(
        (halfway / full - (1.0 + percent / 200.0)).abs() < 0.01,
        "full={full}, halfway={halfway}"
    );
    assert!(
        (execute / full - (1.0 + percent / 100.0)).abs() < 0.01,
        "full={full}, execute={execute}"
    );
    assert!(
        (untalented / full - 1.0).abs() < 0.01,
        "without talent={untalented}, full={full}"
    );
}
