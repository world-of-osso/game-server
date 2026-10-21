//! Native starter-kit rotation receipts; no granted resources or proc auras.
use super::kit_tests::level32;
use super::*;
use crate::spell_cast::powers::{PowerPools, power_value};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use shared::components::{Health, PowerType, ThreatTable};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

const COMBO_ACTIONS: &[u32] = &[100780, 100784, 107428, 113656, 101546, 392983, 322109];

fn dummy(h: &mut Harness, entity: Entity) {
    // Explicit nonlethal training dummy fixture; never modifies actor damage/resources.
    h.app.world_mut().entity_mut(entity).insert(Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    });
    let mut threat = ThreatTable::default();
    threat.add_threat(h.caster.to_bits(), 1.0);
    h.app.world_mut().entity_mut(entity).insert(threat);
}

fn measure(entry: u32, targets: usize, seconds: usize) -> serde_json::Value {
    let (mut h, unit) = level32(entry);
    let primary = h.npc;
    dummy(&mut h, primary);
    for _ in 1..targets {
        let extra = spawn_npc(&mut h.app, 1.0);
        dummy(&mut h, extra);
    }
    let start = h.inbox.log.len();
    let mut capped_frames = BTreeMap::<String, usize>::new();
    let mut aura_frames = BTreeMap::<u32, usize>::new();
    for _ in 0..seconds * 20 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if let Some(pools) = h.app.world().get::<PowerPools>(unit) {
            for pool in &pools.pools {
                if pool.current >= pool.max {
                    *capped_frames
                        .entry(format!("{:?}", pool.power))
                        .or_default() += 1;
                }
            }
        }
        if let Some(auras) = h.app.world().get::<crate::auras::ActiveAuras>(unit) {
            for aura in auras.iter() {
                *aura_frames.entry(aura.spell_id).or_default() += 1;
            }
        }
    }
    let mut casts = BTreeMap::<u32, usize>::new();
    let mut sequence = Vec::new();
    let mut damage = 0_i64;
    for event in &h.inbox.log[start..] {
        if event.source != Some(unit.to_bits()) {
            continue;
        }
        if event.kind == CombatLogKind::CastSuccess {
            if let Some(spell) = event.spell_id {
                *casts.entry(spell).or_default() += 1;
                if COMBO_ACTIONS.contains(&spell) {
                    sequence.push(spell);
                }
            }
        }
        if event.kind == CombatLogKind::Damage {
            damage += i64::from(event.amount);
        }
    }
    let repeats = sequence
        .windows(2)
        .filter(|pair| pair[0] == pair[1])
        .count();
    let report = serde_json::json!({
        "entry":entry,"level":32,"targets":targets,"seconds":seconds,
        "casts":casts,"combo_sequence":sequence,"combo_repeats":repeats,
        "actor_damage":damage,"capped_frames":capped_frames,"aura_frames":aura_frames,
        "frames":seconds*20,"chi_end":power_value(h.app.world(),unit,PowerType::Chi),
        "limits":"Capped frames measure time at cap, not exact lost gains; actor damage excludes guardians. Native proc RNG, no grants."
    });
    println!("ROTATION {report}");
    if let Some(root) = std::env::var_os("ROTATION_OUTPUT") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(format!("{entry}-{targets}.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    report
}

#[test]
fn rotation_fidelity_windwalker_single_target_no_repeat() {
    let report = measure(227734, 1, 60);
    assert!(
        report["combo_sequence"].as_array().unwrap().len() >= 10,
        "sufficient actual casts required"
    );
    assert_eq!(
        report["combo_repeats"], 0,
        "SimC76f8118 combo_strike rejects consecutive eligible actions"
    );
    assert!(
        report["casts"]["107428"].as_u64().unwrap_or(0) > 0,
        "Rising Sun Kick must not be starved by Blackout Kick"
    );
    assert!(
        report["casts"]["113656"].as_u64().unwrap_or(0) > 0,
        "Fists of Fury must not be starved by Blackout Kick"
    );
}

#[test]
fn rotation_fidelity_windwalker_three_target_no_repeat() {
    let report = measure(227734, 3, 60);
    assert!(
        report["combo_sequence"].as_array().unwrap().len() >= 10,
        "sufficient actual casts required"
    );
    assert_eq!(
        report["combo_repeats"], 0,
        "AoE must also preserve combo_strike"
    );
    assert!(
        report["casts"]["113656"].as_u64().unwrap_or(0) > 0,
        "multitarget APL prioritizes Fists of Fury"
    );
}
