//! Assisted recommendations must honor the native Whirling Dragon Punch window.
use super::*;
use game_engine_core::assisted_combat::is_spell_usable;
use std::collections::BTreeSet;

#[test]
#[ignore = "requires native 269 player profile"]
fn assisted_269_wdp_is_unavailable_without_earned_caster_aura() {
    let mut run = prepare(269, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(!crate::auras::has_aura(run.app.world(), actor, 196742));
    let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
        run.app.world_mut(),
        actor,
        target,
        &BTreeSet::from([152175]),
    )
    .unwrap();
    // SimC76f8118 monk/sc_monk.cpp4550–4565: WDP's window requires both source cooldowns.
    assert_eq!(is_spell_usable(152175, &facts), Ok(false));
}
