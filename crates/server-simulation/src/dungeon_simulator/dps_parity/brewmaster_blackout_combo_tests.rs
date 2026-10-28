//! Blackout Combo follows actual casts, not unrelated melee proc events.
use super::*;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn blackout_kick_grants_combo_and_tiger_palm_consumes_it() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
        .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 228563),
        "Blackout Kick grants Blackout Combo"
    );
    for _ in 0..24 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
        .unwrap();
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 228563),
        "Tiger Palm consumes Blackout Combo"
    );
    for _ in 0..24 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 228563),
        "queued Tiger Palm impact must not re-grant Blackout Combo"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .unwrap();
    for _ in 0..24 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 228563),
        "Keg Smash must not grant Blackout Combo"
    );
}
