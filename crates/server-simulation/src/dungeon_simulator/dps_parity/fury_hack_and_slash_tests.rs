//! Hack and Slash boosts one complete native Raging Blow weapon sequence.
use super::literal_rotation::execution::{Execution, execute};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const HACK_AND_SLASH: u32 = 383873;

#[derive(Debug)]
struct RagingResult {
    damage: [i64; 2],
    hack_remaining: bool,
    initial_charges: u8,
    native_bonus: f32,
}

fn cast_raging_blow(grant_hack: bool) -> RagingResult {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(72, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 383877));
    assert!(!crate::auras::has_aura(run.app.world(), actor, HACK_AND_SLASH));
    let native_bonus = crate::spell_cast::scripts::script_effect_value(
        run.app.world(), actor, HACK_AND_SLASH, 0,
    );
    let initial_charges = if grant_hack {
        assert!(crate::spell_cast::cast_triggered(
            run.app.world_mut(), actor, HACK_AND_SLASH, Some(actor), 1,
        ));
        run.app.world().get::<crate::auras::ActiveAuras>(actor).unwrap()
            .iter().find(|aura| aura.spell_id == HACK_AND_SLASH).unwrap().charges
    } else {
        0
    };
    execute(
        run.app.world_mut(), actor, Execution::Spell(85288, primary, Some(primary)),
    ).unwrap();
    for _ in 0..8 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage = [96103, 85384].map(|spell| {
        events.iter().filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(primary.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
        }).map(|event| i64::from(event.amount)).sum()
    });
    assert!(damage.iter().all(|amount| *amount > 0), "both native weapon hits: {damage:?}");
    RagingResult {
        damage,
        hack_remaining: crate::auras::has_aura(run.app.world(), actor, HACK_AND_SLASH),
        initial_charges,
        native_bonus,
    }
}

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_hack_and_slash_is_consumed_after_both_raging_blow_weapons() {
    let plain = cast_raging_blow(false);
    let boosted = cast_raging_blow(true);
    assert!(!plain.hack_remaining, "Raging Blow cannot produce Rampage's Hack proc: {plain:?}");
    assert!(!boosted.hack_remaining,
        "SimC5061 consumes the next-use buff after both native weapons: {boosted:?}");
    let multiplier = 1.0 + f64::from(boosted.native_bonus) / 100.0;
    for (normal, buffed) in plain.damage.into_iter().zip(boosted.damage) {
        let ratio = buffed as f64 / normal as f64;
        assert!((ratio - multiplier).abs() < 0.01,
            "both weapon hits retain the native bonus before consumption: {plain:?}, {boosted:?}");
    }
    assert_eq!(boosted.initial_charges, 0, "native next-use semantics are not proc charges");
}
