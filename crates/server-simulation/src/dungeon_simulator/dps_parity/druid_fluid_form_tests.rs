//! Selected Fluid Form shifts on actual Rake and Mangle casts.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Fluid Form profile and native world data"]
fn fluid_form_shifts_rake_and_mangle_on_real_casts() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .has_spell(449193)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1822, Some(target), false)
        .unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(768));
    assert!(!auras.has_spell(5487));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
        .unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(5487));
    assert!(!auras.has_spell(768));
}
