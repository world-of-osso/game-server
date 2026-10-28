//! Incarnation must enter Bear Form from the real Cat Form opener.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires Guardian profile and native world data"]
fn guardian_incarnation_shifts_cat_to_bear_before_thrash() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .has_spell(768)
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 102558, Some(actor), false)
        .unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(5487));
    assert!(!auras.has_spell(768));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
}
