//! Fire Nova must burst from actual own Flame Shocks, not pass because Burning Core also ticks Nova.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn nova_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let owned = event.source == Some(run.actor.to_bits()) && event.spell_id == Some(333977);
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            owned && damage
        })
        .count()
}

fn cast_blaze_and_count_immediate_nova_hits(run: &mut Run) -> usize {
    let target = run.targets[0];
    for _ in 0..1200 {
        // Reset after every readiness wait: periodic tier ticks cannot count as this execute's burst.
        let before = nova_hits(run);
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            470057,
            Some(target),
            false,
        ) {
            Ok(()) => return nova_hits(run) - before,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native Voltaic Blaze for Fire Nova: {reason:?}"),
        }
    }
    panic!("real Voltaic Blaze did not become ready");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn native_fire_nova_proc_explodes_each_own_flame_shock_in_the_actual_blaze_execute() {
    // SimC76f8118 sc_shaman.cpp10123–10130/7043–7076; native1260666 owns the actual30% roll.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(1260666)
    );
    engage(&mut run);
    cast_blaze_and_count_immediate_nova_hits(&mut run);
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    for &target in &run.targets {
        assert!(
            run.app
                .world()
                .get::<ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == 188389 && aura.caster == run.actor),
            "actual first Voltaic Blaze must establish its own native Flame Shock"
        );
    }
    let mut burst = 0;
    for _ in 0..20 {
        burst = cast_blaze_and_count_immediate_nova_hits(&mut run);
        if burst > 0 {
            break;
        }
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    assert_eq!(
        burst, 9,
        "selected Fire Nova earns a real immediate three-by-three burst, independent of periodic tier Nova ticks"
    );
}
