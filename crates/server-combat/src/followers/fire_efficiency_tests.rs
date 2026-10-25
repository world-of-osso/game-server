//! Controlled naked70 Fire receipts; observational baseline, not a parity oracle.
use super::rotation_dummy_tests::{prepare_dummies, prepare_follower, snapshot_stats};
use bevy::prelude::*;
use serde::Serialize;
use shared::casting::CastState;
use shared::components::{Level, UnitLevel};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

const FIREBALL: u32 = 133;
const SECONDS: usize = 60;
const FRAMES: usize = SECONDS * 20;

#[derive(Default, Serialize)]
struct SpellTotals {
    casts: usize,
    damage_events: usize,
    damage: i64,
}

fn observe_clock(world: &World, caster: Entity, begin: f64) -> serde_json::Value {
    let now = crate::spell_cast::history_now(world);
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(FIREBALL)
        .unwrap();
    let gcd = world
        .get::<crate::spell_cast::SpellHistory>(caster)
        .is_some_and(|history| history.on_gcd(spell, now));
    serde_json::json!({
        "time": now - begin,
        "cast_spell": world.get::<CastState>(caster).map(|cast| cast.spell_id),
        "fireball_gcd": gcd,
    })
}

fn starter_config(world: &World) -> serde_json::Value {
    let classes = world.resource::<crate::spell_info::ClassSpells>();
    let class = classes
        .classes
        .iter()
        .find(|class| class.class_id == 8)
        .unwrap();
    let context = crate::trait_config::TraitContext {
        spec_id: 63,
        level: 70,
    };
    let purchases = crate::trait_config::starter_purchases(&class.traits, context).unwrap();
    let config = crate::trait_config::build_config(&class.traits, context, &purchases).unwrap();
    let entries: Vec<_> = config
        .iter()
        .map(|entry| {
            serde_json::json!({
                "node_id": entry.node_id, "entry_id": entry.entry_id,
                "ranks": entry.ranks, "granted": entry.granted,
            })
        })
        .collect();
    entries.into()
}

fn summarize_events(
    h: &crate::spell_cast::test_harness::Harness,
    caster: Entity,
    start: usize,
) -> BTreeMap<u32, SpellTotals> {
    let mut totals = BTreeMap::<u32, SpellTotals>::new();
    for event in &h.inbox.log[start..] {
        if event.source != Some(caster.to_bits()) {
            continue;
        }
        let Some(spell) = event.spell_id else {
            continue;
        };
        let total = totals.entry(spell).or_default();
        match event.kind {
            CombatLogKind::CastSuccess => total.casts += 1,
            CombatLogKind::Damage => {
                total.damage_events += 1;
                total.damage += i64::from(event.amount);
            }
            _ => {}
        }
    }
    totals
}

#[test]
fn fire_efficiency_native70_ordinary_fireball_earns_native_ignite() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    let targets = prepare_dummies(&mut h, 1);
    let target = targets[0];
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Level(70), UnitLevel(70)));
    let mastery = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12846)
        .unwrap();
    assert!(
        mastery
            .effects
            .iter()
            .any(|effect| effect.index == 0 && effect.amount > 0.0)
    );
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    h.advance(100);
    assert!(
        h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(FIREBALL)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "ordinary native Fireball must land before testing its earned consequence"
    );
    assert!(
        h.inbox.log[start..].iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(12654)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        }),
        "native12846 positive ordinary hit must earn actual12654 periodic damage"
    );
}

/// Test-only script isolation: retain native70/mastery, remove only the new bridge.
/// Unmodified native Intensifying Flame→Ignite feedback has its own integration case.
fn isolate_ignite_producer(h: &mut crate::spell_cast::test_harness::Harness, caster: Entity) {
    let instance = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 416714)
        .expect("actual selected native416714")
        .instance_id;
    crate::auras::remove_instance(
        h.app.world_mut(),
        caster,
        instance,
        crate::auras::RemoveReason::Cancelled,
    );
    assert!(crate::auras::has_aura(h.app.world(), caster, 12846));
    assert!(!crate::auras::has_aura(h.app.world(), caster, 416714));
}

#[test]
fn fire_efficiency_native70_ignite_uses_native_share_and_cannot_feed_itself() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    isolate_ignite_producer(&mut h, caster);
    let target = prepare_dummies(&mut h, 1)[0];
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Level(70), UnitLevel(70)));
    let percent = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12846)
        .unwrap()
        .effects[0]
        .amount;
    let dot = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(12654)
        .unwrap();
    let period = dot
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    let ticks = dot.misc.as_ref().unwrap().duration.unwrap().duration_ms as u32 / period;
    assert_eq!((period, ticks), (1000, 9), "native12654 clock");
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    h.advance(100);
    let damage = h.inbox.log[start..]
        .iter()
        .find(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIREBALL)
                && event.kind == CombatLogKind::Damage
        })
        .unwrap()
        .amount;
    let ignite = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12654 && aura.caster == caster)
        .unwrap();
    let amount = ignite
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(amount, damage as f32 * percent / 100.0 / ticks as f32);
    h.advance(200);
    let hits: Vec<_> = h.inbox.log[start..]
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(12654)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .collect();
    assert_eq!(
        hits.len(),
        ticks as usize,
        "one earned9tick bank, no self-proc cascade"
    );
    assert!(hits.iter().all(|event| event.amount == amount as i32));
    assert!(!crate::auras::has_aura(h.app.world(), target, 12654));
}

#[test]
fn fire_efficiency_native70_consumed_hot_streak_doubles_ignite_on_impact() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    isolate_ignite_producer(&mut h, caster);
    let targets = prepare_dummies(&mut h, 2);
    for target in &targets {
        h.app
            .world_mut()
            .entity_mut(*target)
            .insert((Level(70), UnitLevel(70)));
    }
    for _ in 0..2 {
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            caster,
            108853,
            Some(targets[0]),
            false,
        )
        .unwrap();
        // Native108853 RecoveryTime500ms remains active between its charges.
        h.advance(10);
    }
    h.advance(3);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, 48108),
        "two real FireBlast crits earn HotStreak"
    );
    let percent = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12846)
        .unwrap()
        .effects[0]
        .amount;
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 11366, Some(targets[1]), false)
        .unwrap();
    assert!(
        h.app.world().get::<CastState>(caster).is_none(),
        "admitted instant Pyroblast"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, 48108),
        "spent before missile impact"
    );
    h.advance(60);
    let damage = h.inbox.log[start..]
        .iter()
        .find(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(targets[1].to_bits())
                && event.spell_id == Some(11366)
                && event.kind == CombatLogKind::Damage
        })
        .unwrap()
        .amount;
    let ignite = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(targets[1])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12654 && aura.caster == caster)
        .unwrap();
    let amount = ignite
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(
        amount,
        damage as f32 * (percent * 2.0) / 100.0 / 9.0,
        "native HotStreak share survives its consumed aura and ordinary missile"
    );
}

#[test]
#[ignore = "native Flamestrike script-isolation gate"]
fn fire_flamestrike_2120_spends_actual_earned_hot_streak_at_instant_admission() {
    assert_flamestrike_spends_earned_hot_streak(2120);
}

#[test]
#[ignore = "native Flamestrike script-isolation gate"]
fn fire_flamestrike_1254851_spends_actual_earned_hot_streak_at_instant_admission() {
    assert_flamestrike_spends_earned_hot_streak(1254851);
}

fn assert_flamestrike_spends_earned_hot_streak(spell_id: u32) {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    let target = prepare_dummies(&mut h, 1)[0];
    for _ in 0..2 {
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, 108853, Some(target), false)
            .unwrap();
        h.advance(10);
    }
    h.advance(3);
    assert!(
        crate::auras::has_aura(h.app.world(), caster, 48108),
        "two nativeFireBlast crits earn the real proc"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, spell_id, Some(target), false)
        .unwrap();
    assert!(
        h.app.world().get::<CastState>(caster).is_none(),
        "native{spell_id} admitted instant with earned Hot Streak"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, 48108),
        "ordinary{spell_id} spends its admitted Hot Streak at launch"
    );
}

fn land_fireball(
    h: &mut crate::spell_cast::test_harness::Harness,
    caster: Entity,
    target: Entity,
) -> i32 {
    let start = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    for _ in 0..100 {
        h.advance(1);
        if let Some(event) = h.inbox.log[start..].iter().find(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(FIREBALL)
                && event.kind == CombatLogKind::Damage
        }) {
            return event.amount;
        }
    }
    panic!("ordinary native Fireball did not land within5s");
}

#[test]
fn fire_efficiency_native70_ordinary_ignite_refresh_keeps_unpaid_native_damage() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    isolate_ignite_producer(&mut h, caster);
    let target = prepare_dummies(&mut h, 1)[0];
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Level(70), UnitLevel(70)));
    land_fireball(&mut h, caster, target);
    let old = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12654 && aura.caster == caster)
        .unwrap()
        .clone();
    let old_amount = old
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let percent = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12846)
        .unwrap()
        .effects[0]
        .amount;
    let start = h.inbox.log.len();
    let second_damage = land_fireball(&mut h, caster, target);
    let paid = h.inbox.log[start..]
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(12654)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .count();
    assert!(
        paid > 0 && paid < 9,
        "actual old bank paid ticks before refresh: {paid}"
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 12654 && aura.caster == caster)
        .unwrap();
    let refreshed = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    let new_amount = second_damage as f32 * percent / 100.0 / 9.0;
    let unpaid = old_amount * (9 - paid) as f32;
    assert_eq!(
        refreshed,
        new_amount + unpaid / 9.0,
        "ordinary producer must enter the shared nativeROLLING_PERIODIC carry once"
    );
    assert!(refreshed > new_amount);
}

#[test]
#[ignore = "measurement: explicit FIRE_EFFICIENCY_OUTPUT and frozen loadout required"]
fn fire_efficiency_native70_single_target_60s() {
    let output = std::env::var("FIRE_EFFICIENCY_OUTPUT").unwrap();
    let frozen = std::env::var("FIRE_EFFICIENCY_LOADOUTS").unwrap();
    let _seed = crate::random::seed_scope(1);
    let (mut h, caster) = prepare_follower(209059, 63, 70);
    let targets = prepare_dummies(&mut h, 1);
    for target in &targets {
        h.app
            .world_mut()
            .entity_mut(*target)
            .insert((Level(70), UnitLevel(70)));
    }
    let loadouts: serde_json::Value =
        serde_json::from_slice(&std::fs::read(frozen).unwrap()).unwrap();
    let expected = loadouts["loadouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["spec_id"] == 63 && row["level"] == 70)
        .unwrap();
    let config = starter_config(h.app.world());
    assert_eq!(
        config, expected["config"],
        "same native starter inputs as supplied SimC profile"
    );
    let known: Vec<_> = h
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(caster)
        .unwrap()
        .iter()
        .collect();
    let begin = crate::spell_cast::history_now(h.app.world());
    let start = h.inbox.log.len();
    let initial_stats = snapshot_stats(&h, caster);
    let mut frames = Vec::with_capacity(FRAMES);
    let mut stats = Vec::new();
    let mut previous_stats = serde_json::Value::Null;
    for _ in 0..FRAMES {
        let before = observe_clock(h.app.world(), caster, begin);
        let first = h.inbox.go.len();
        super::update_ai(h.app.world_mut());
        let after_ai = observe_clock(h.app.world(), caster, begin);
        h.advance(1);
        let after = observe_clock(h.app.world(), caster, begin);
        let casts: Vec<_> = h.inbox.go[first..]
            .iter()
            .filter(|event| event.caster == caster.to_bits())
            .map(|event| event.spell_id)
            .collect();
        frames.push(serde_json::json!({"before": before, "after_ai": after_ai, "after": after, "receipts": casts}));
        let current = snapshot_stats(&h, caster);
        if current != previous_stats {
            stats.push(serde_json::json!({"time": crate::spell_cast::history_now(h.app.world()) - begin, "stats": current}));
            previous_stats = current;
        }
    }
    let elapsed = crate::spell_cast::history_now(h.app.world()) - begin;
    assert!(
        (elapsed - SECONDS as f64).abs() < 0.001,
        "actual60s clock: {elapsed}"
    );
    let totals = summarize_events(&h, caster, start);
    let damage: i64 = totals.values().map(|total| total.damage).sum();
    assert!(damage > 0, "actual native Fire damage");
    let retaliation: i64 = h.inbox.log[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage && event.source == Some(targets[0].to_bits())
        })
        .map(|event| i64::from(event.amount))
        .sum();
    assert_eq!(
        retaliation, 0,
        "stationary non-retaliating reference target"
    );
    let report = serde_json::json!({
        "spec": 63, "level": 70, "race": 0, "equipment_items": 0,
        "seconds": elapsed, "seed": 1, "targets": 1,
        "native_starter_config": config, "known_spells": known,
        "expected_native_source_book": expected["known"],
        "initial_stats": initial_stats, "stat_transitions": stats,
        "per_spell": totals, "damage": damage, "dps": damage as f64 / elapsed,
        "frames": frames, "dummy_retaliation_damage": retaliation,
        "limits": ["single native seed, not a statistical parity claim", "constant-HP native training dummy differs from SimC fixed-time execute health", "live KnownSpells is recorded without injecting the exported source book"],
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    serde_json::to_writer_pretty(&mut file, &report).unwrap();
    println!("FIRE_NATIVE70 DPS={}", report["dps"]);
}
