//! TC a352b1fa spell_paladin.cpp739–804: accumulate real damage, discharge on expiry only.
use super::ret_purpose_tests::earn_finisher_power;
use super::ret_tier_tests::native_ret;
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

const SENTENCE: u32 = 343527;
const SENTENCE_DAMAGE: u32 = 387113;
const JUDGMENT: u32 = 20271;

fn owned_sentence(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.spell_id == SENTENCE && aura.caster == run.actor)
}

fn damage(run: &Run, spell: u32) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell)
        })
        .filter(|event| event.kind == CombatLogKind::Damage)
        .map(|event| i64::from(event.amount))
        .sum()
}

fn cast_ready(run: &mut Run, spell: u32) {
    for _ in 0..300 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => return,
            Err(
                CastFailure::OnCooldown
                | CastFailure::OnGlobalCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native Sentence fixture spell{spell}: {reason:?}"),
        }
    }
    panic!("native Sentence fixture spell{spell} did not become ready");
}

fn funded_sentence() -> Run {
    let mut run = native_ret(4);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    earn_finisher_power(&mut run, SENTENCE);
    cast_ready(&mut run, SENTENCE);
    for _ in 0..100 {
        run.app.update();
        if owned_sentence(&run).is_some() {
            break;
        }
    }
    assert!(
        owned_sentence(&run).is_some(),
        "ordinary paid Sentence must apply its owned native aura"
    );
    run
}

fn damage_during_sentence(run: &mut Run) -> f64 {
    let expires = owned_sentence(run)
        .unwrap()
        .expires_at
        .expect("native aura has a finite expiry");
    let before = damage(run, JUDGMENT);
    cast_ready(run, JUDGMENT);
    for _ in 0..100 {
        run.app.update();
        if damage(run, JUDGMENT) > before {
            break;
        }
    }
    assert!(
        damage(run, JUDGMENT) > before,
        "actual damage must reach the aura's target"
    );
    assert!(
        owned_sentence(run).is_some(),
        "setup damage must occur during its actual lifetime"
    );
    assert_eq!(
        damage(run, SENTENCE_DAMAGE),
        0,
        "Sentence must not detonate before expiry"
    );
    expires
}

fn wait_past_expiry(run: &mut Run, expires: f64) {
    for _ in 0..600 {
        run.app.update();
        if crate::spell_cast::now(run.app.world()) > expires + 1.0 {
            return;
        }
    }
    panic!("native clock never crossed source-authored Sentence expiry");
}

#[test]
#[ignore = "native Sentence paid cast and real accumulated damage must detonate on expiry"]
fn dps_parity_ret_sentence_native_expiry_dispatches_real_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = funded_sentence();
    let expires = damage_during_sentence(&mut run);
    wait_past_expiry(&mut run, expires);
    assert!(owned_sentence(&run).is_none());
    assert!(
        damage(&run, SENTENCE_DAMAGE) > 0,
        "TC aura expiry must cast actual387113 using accumulated native damage"
    );
}

#[test]
#[ignore = "native Sentence cancelled after actual damage must not discharge later"]
fn dps_parity_ret_sentence_native_cancellation_does_not_dispatch_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = funded_sentence();
    let expires = damage_during_sentence(&mut run);
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.targets[0],
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SENTENCE && aura.caster == actor,
    );
    assert!(owned_sentence(&run).is_none());
    wait_past_expiry(&mut run, expires);
    assert_eq!(damage(&run, SENTENCE_DAMAGE), 0);
}
