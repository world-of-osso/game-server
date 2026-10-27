//! Native Devourer Void Ray admission and per-tick resource boundary.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_void_ray_requires_full_fury_without_spending_it_upfront() {
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .unwrap();
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 99.0)
        .unwrap();
    assert!(
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            473728,
            Some(target)
        )
        .is_err(),
        "Void Ray must not start below 100 Fury outside Meta"
    );
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, 1.0)
        .unwrap();
    // SimC76f8118 sc_demon_hunter.cpp6431–6455: 100 Fury is admission,
    // and resource is spent per tick, not upfront.
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473728,
        Some(target),
    )
    .expect("native Void Ray at 100 Fury");
    let fury = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert_eq!(
        fury, 100.0,
        "Void Ray must retain Fury until its first paid tick"
    );
}
