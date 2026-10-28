//! Black Ox Brew restores real expended Brew charges.
use super::*;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn black_ox_restores_purifying_and_infusion_charges() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    for _ in 0..2 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
            .is_err()
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
            .is_err()
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115399, Some(actor), false)
        .unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    for spell in [119582, 1241059, 119582] {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(actor), false)
            .expect("Black Ox Brew must restore the spent charge");
        for _ in 0..40 {
            run.app.update();
        }
    }
}
