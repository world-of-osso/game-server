//! Real Empty Barrel and Exploding Keg restore Keg Smash readiness.
use super::*;

fn spend_kegs(run: &mut Run) {
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..2 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .unwrap();
        for _ in 0..24 {
            run.app.update();
        }
    }
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .is_err(),
        "both Keg Smash charges are recharging"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn empty_barrel_grant_restores_keg_smash() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    spend_kegs(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    assert!(crate::auras::has_aura(run.app.world(), actor, 1265307));
    for _ in 0..24 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .expect("selected Bring Me Another restores Keg Smash");
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn exploding_keg_restores_keg_smash() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    spend_kegs(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 325153, Some(target), false)
        .unwrap();
    for _ in 0..24 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
        .expect("Exploding Keg restores Keg Smash");
}
