//! Bring Me Another's native proc only admits Brew casts.
use super::*;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn unrelated_monk_casts_do_not_grant_empty_barrel() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..20 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1265307),
            "Tiger Palm must not trigger Bring Me Another"
        );
    }
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn purifying_brew_can_trigger_empty_barrel() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    for _ in 0..12 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        if crate::auras::has_aura(run.app.world(), actor, 1265307) {
            return;
        }
        for _ in 0..500 {
            run.app.update();
        }
    }
    panic!("seeded actual Purifying Brew casts must include a Bring Me Another proc");
}
