//! Gore is earned by a landed Thrash and consumed by Mangle.
use super::*;
use crate::auras::{self, ActiveAuras};
use shared::components::PowerType;

#[test]
#[ignore = "requires Guardian Gore profile and native world data"]
fn guardian_thrashes_earn_gore_for_the_next_mangle() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let instance = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 210706 && a.caster == actor)
        .unwrap()
        .instance_id;
    assert!(auras::set_effect_amount(
        run.app.world_mut(),
        actor,
        instance,
        0,
        100.0
    ));
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 77758, Some(target), false)
        .unwrap();
    assert!(auras::has_aura(run.app.world(), actor, 93622));
    for _ in 0..40 {
        run.app.update();
    }
    let before = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 33917, Some(target), false)
        .unwrap();
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Rage)
        .unwrap()
        .0;
    assert!(!auras::has_aura(run.app.world(), actor, 93622));
    assert!(
        after - before >= 200.0,
        "Gore increases actual Mangle rage: {before} -> {after}"
    );
}
