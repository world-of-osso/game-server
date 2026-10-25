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

fn counter(run: &Run) -> f32 {
    owned_sentence(run)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount
}

fn assert_native_accumulation(run: &Run, initial: f32, start: usize) {
    let percent = owned_sentence(run)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert_eq!(
        percent, 20.0,
        "native343527 effect1 is the percentage authority"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[start..];
    let eligible: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .filter(|event| event.school_mask & 1 == 0 && event.amount > 0)
        .collect();
    let logged: f32 = eligible.iter().map(|event| event.amount as f32).sum();
    let expected = logged * percent / 100.0;
    // CombatLogEvent truncates each native float amount to its wire integer.
    let truncation_bound = (eligible.len() as f32 + 1.0) * percent / 100.0;
    let actual = counter(run) - initial;
    assert!(
        (actual - expected).abs() <= truncation_bound,
        "native accumulation={actual}, logged-derived={expected}, events={events:?}"
    );
}

#[test]
#[ignore = "native Sentence native percentage includes own Holy damage, excludes physical whites"]
fn dps_parity_ret_sentence_native_accumulation_excludes_actual_white_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = funded_sentence();
    // Isolate physical whites from the profile's native Aura361 replacement.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.effects.iter().any(|effect| effect.aura_type == 361),
    );
    let initial = counter(&run);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    damage_during_sentence(&mut run);
    for _ in 0..80 {
        run.app.update();
        if run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(owned_sentence(&run).is_some());
    assert!(
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "real physical white damage must exercise the exclusion"
    );
    assert_native_accumulation(&run, initial, start);
}

#[test]
#[ignore = "native Sentence must not gather an unrelated real actor's Holy damage"]
fn dps_parity_ret_sentence_native_accumulation_excludes_foreign_actor_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = funded_sentence();
    let copy = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.actor)
        .unwrap();
    let other = crate::followers::simulator::spawn_parity_attacker(
        run.app.world_mut(),
        70,
        90,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    inputs::equip(run.app.world_mut(), other, &run.profile).unwrap();
    inputs::install_traits(run.app.world_mut(), other, &[]).unwrap();
    literal_rotation::capture_fixture(
        run.app.world_mut(),
        other,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
    )
    .unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    let initial = counter(&run);
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        other,
        JUDGMENT,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..80 {
        run.app.update();
        if run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .any(|event| {
                event.source == Some(other.to_bits())
                    && event.spell_id == Some(JUDGMENT)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(
        run.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .any(|event| {
                event.source == Some(other.to_bits())
                    && event.spell_id == Some(JUDGMENT)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "foreign native Holy damage must reach the same real target"
    );
    assert_native_accumulation(&run, initial, start);
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
