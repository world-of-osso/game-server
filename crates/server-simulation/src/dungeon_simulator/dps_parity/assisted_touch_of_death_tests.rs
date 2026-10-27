//! Assisted recommendations respect actual Touch of Death target admission.
use super::*;
use game_engine_core::assisted_combat::is_spell_usable;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires native 269 player profile and frozen assisted combat data"]
fn assisted_touch_of_death_unavailable_until_target_is_eligible() {
    let mut run = prepare(269, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(run.app.world().get::<shared::components::Player>(actor).is_some());
    assert!(crate::auras::has_aura(run.app.world(), actor, 322113));
    let needed = BTreeSet::from([322109]);
    let mut rotation = player_rotation::PlayerRotation::load(269, 0.0).unwrap();
    let health = run.app.world().get::<shared::components::Health>(target).unwrap();
    assert!(health.current > run.app.world().get::<shared::components::Health>(actor).unwrap().max);
    let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
        run.app.world_mut(), actor, target, &needed,
    ).unwrap();
    assert_eq!(is_spell_usable(322109, &facts), Ok(false));
    rotation.drive(run.app.world_mut(), actor, target).unwrap();

    // SimC76f8118 monk/sc_monk.cpp2675–2681 permits the improved rank below its threshold.
    let mut health = run.app.world_mut().get_mut::<shared::components::Health>(target).unwrap();
    health.current = health.max * 0.10;
    let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
        run.app.world_mut(), actor, target, &needed,
    ).unwrap();
    assert_eq!(is_spell_usable(322109, &facts), Ok(true));
}
