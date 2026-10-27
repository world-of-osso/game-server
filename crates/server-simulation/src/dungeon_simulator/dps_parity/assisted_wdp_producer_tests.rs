//! The player earns the native WDP window through real source casts.
use super::*;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, spell: u32) {
    for _ in 0..1200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(), run.actor, spell, Some(run.targets[0]), false,
        ) {
            Ok(()) => { run.app.update(); return; }
            Err(crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::NotEnoughPower(_)) => run.app.update(),
            Err(reason) => panic!("native player spell {spell}: {reason:?}"),
        }
    }
    panic!("native player spell {spell} did not become ready");
}

#[test]
#[ignore = "requires native 269 player profile"]
fn assisted_269_player_earns_wdp_from_real_cooldowns() {
    let mut run = prepare_literal(269, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    assert!(run.app.world().get::<shared::components::Player>(actor).is_some());
    assert!(!crate::auras::has_aura(run.app.world(), actor, 196742));
    for spell in [100780, 100780, 107428, 100780, 100780, 113656] {
        cast_when_ready(&mut run, spell);
    }
    // SimC76f8118 monk/sc_monk.cpp4550–4565: overlapping RSK/FoF recovery earns WDP.
    assert!(crate::auras::has_aura(run.app.world(), actor, 196742));
    let events = &run.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    for spell in [107428, 113656] {
        assert!(events.iter().any(|event| event.source == Some(actor.to_bits()) && event.spell_id == Some(spell) && event.kind == CombatLogKind::CastSuccess));
    }
}
