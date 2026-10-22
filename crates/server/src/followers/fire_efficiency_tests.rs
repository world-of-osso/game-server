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
