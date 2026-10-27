//! Native Void Ray channel tick Fury spending.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_spends_fury_for_each_native_tick() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    run.app.init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 1217607), "out-of-Meta fixture");
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0).expect("native Fury pool");
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 100.0).expect("native Fury pool");
    crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 473728, Some(target)).expect("native Void Ray admission at 100 Fury");
    let initial = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury).unwrap().0;
    assert_eq!(initial, 100.0, "native channel must not spend Fury upfront");
    for _ in 0..40 {
        if ray_ticks(&run, actor) >= 3 { break; }
        run.app.update();
    }
    let ticks = ray_ticks(&run, actor);
    assert!(ticks >= 3, "real native Void Ray child ticks must execute");
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury).unwrap().0;
    // SimC76f8118 sc_demon_hunter.cpp6431–6455 pays native1213649's
    // 70448 cost of 5 Fury for each non-Meta channel tick.
    assert_eq!(after, initial - ticks as f32 * 5.0, "every real Void Ray tick must pay its native Fury cost");
}

fn ray_ticks(run: &Run, actor: Entity) -> usize {
    run.app.world().resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>().0.iter().filter(|(caster, spell, _)| *caster == actor && *spell == 1213649).count()
}
