//! Concrete native max-level selection and Voltaic Blaze boundaries, not full APL parity.
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, ThreatTable};
use shared::protocol::CombatLogKind;

fn prepare(entry: u32, count: usize) -> (Harness, Entity, Vec<Entity>) {
    prepare_at_level(entry, count, 90)
}

fn prepare_at_level(entry: u32, count: usize, level: u8) -> (Harness, Entity, Vec<Entity>) {
    let (mut h, caster) = at_level(entry, level, |_| {});
    let mut targets = vec![h.npc];
    targets.extend((1..count).map(|index| spawn_npc(&mut h.app, index as f32)));
    for &target in &targets {
        let mut threat = ThreatTable::default();
        threat.add_threat(h.caster.to_bits(), 1.0);
        h.app.world_mut().entity_mut(target).insert((
            Health {
                current: 1_000_000_000.0,
                max: 1_000_000_000.0,
            },
            threat,
        ));
    }
    (h, caster, targets)
}

#[test]
fn rotation_selection_262_native_target_earthquake_deals_periodic_damage() {
    let (mut h, caster, targets) = prepare_at_level(209053, 3, 70);
    assert!(
        h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(caster)
            .unwrap()
            .contains(462620)
    );
    let first = h.inbox.log.len();
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 462620, Some(target), false)
        .unwrap();
    h.advance(60);
    assert!(
        h.inbox.log[first..].iter().any(|event| {
            event.kind == CombatLogKind::CastSuccess
                && event.spell_id == Some(462620)
                && event.source == Some(caster.to_bits())
        }),
        "real selected native target Earthquake must complete before checking its consequences"
    );
    // No further AI casts: let the actual native6s field run to natural expiry.
    h.advance(140);
    assert!(targets.iter().all(|&target| h.health(target) > 0.0));
    let hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(77478)
                && event.source == Some(caster.to_bits())
                && event.amount > 0
        })
        .collect();
    assert!(
        !hits.is_empty(),
        "native462620 explicitly names actual77478 damage but completed casts currently create no damaging field"
    );
}

#[test]
fn rotation_selection_262_native_ascendance_entry_hits_six_enemies() {
    let (mut h, caster, targets) = prepare(209053, 7);
    for &target in &targets {
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, target);
    }
    assert!(targets.iter().all(|&target| {
        h.app
            .world()
            .get::<shared::components::CombatStatus>(target)
            .is_some_and(|status| status.0)
    }));
    let idle = spawn_npc(&mut h.app, 0.25);
    let distant = spawn_npc(&mut h.app, 80.0);
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, distant);
    assert!(
        targets
            .iter()
            .all(|&target| !crate::auras::has_aura(h.app.world(), target, 188389,))
    );
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 1219480, Some(caster), false)
        .unwrap();
    h.advance(20);
    assert!(crate::auras::has_aura(h.app.world(), caster, 1219480));
    let shocked: Vec<_> = targets
        .iter()
        .copied()
        .filter(|&target| {
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 188389 && aura.caster == caster)
                })
        })
        .collect();
    assert_eq!(
        shocked.len(),
        6,
        "native1219480s7 must apply six actual owned Flame Shocks"
    );
    let burst_hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
        })
        .collect();
    assert_eq!(
        burst_hits.len(),
        6,
        "entry must launch real native Lava Burst damage once per selected enemy"
    );
    assert!(shocked.iter().all(|target| {
        burst_hits
            .iter()
            .any(|hit| hit.target == Some(target.to_bits()))
    }));
    for excluded in [idle, distant] {
        assert!(!crate::auras::has_aura(h.app.world(), excluded, 188389));
        assert!(
            burst_hits
                .iter()
                .all(|hit| hit.target != Some(excluded.to_bits()))
        );
    }
}

#[test]
fn rotation_selection_262_native_ascendance_entry_uses_half_lava_burst_damage() {
    let (mut h, caster, _) = prepare(209053, 1);
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 188389, Some(h.npc), 1);
    h.advance(1);
    let first = h.inbox.log.len();
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 285452, Some(h.npc), 1);
    h.advance(1);
    let full = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("native ordinary Lava Burst control must actually damage its Flame-Shocked target")
        .clone();
    assert!(full.crit && full.amount > 0);
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 1219480, Some(caster), false)
        .unwrap();
    h.advance(20);
    assert!(crate::auras::has_aura(h.app.world(), caster, 1219480));
    let half = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("native Ascendance must launch actual Lava Burst damage");
    assert!(
        half.crit,
        "entry Flame Shock precedes the Lava Burst crit decision"
    );
    let damage = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(285452)
        .unwrap();
    let variance = f64::from(damage.effects[0].variance) / 2.0;
    // Independent real variance draws, not paired RNG: both native rounding bounds count.
    let minimum = ((f64::from(full.amount) - 0.5) * 0.5 * (1.0 - variance) / (1.0 + variance) - 0.5)
        .floor() as i32;
    let maximum = ((f64::from(full.amount) + 0.5) * 0.5 * (1.0 + variance) / (1.0 - variance) + 0.5)
        .ceil() as i32;
    assert!(
        (minimum..=maximum).contains(&half.amount),
        "native1219480s10=50%: full={}, half={}, expected={minimum}..={maximum}",
        full.amount,
        half.amount
    );
    let first = h.inbox.log.len();
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, 285452, Some(h.npc), 1);
    h.advance(1);
    let ordinary = h.inbox.log[first..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(285452)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .expect("ordinary native Lava Burst still damages during Ascendance");
    assert!(ordinary.crit);
    let minimum =
        ((f64::from(full.amount) - 0.5) * (1.0 - variance) / (1.0 + variance) - 0.5).floor() as i32;
    let maximum =
        ((f64::from(full.amount) + 0.5) * (1.0 + variance) / (1.0 - variance) + 0.5).ceil() as i32;
    assert!(
        (minimum..=maximum).contains(&ordinary.amount),
        "only the entry cast is half-effectiveness; ordinary damage={}",
        ordinary.amount
    );
}

#[test]
fn rotation_selection_262_voltaic_blaze_damage_precedes_delayed_flame_shock() {
    let (mut h, caster, targets) = prepare(209053, 3);
    let begin = aura_now(h.app.world());
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 470057, Some(h.npc), false)
        .unwrap();
    h.advance(1);
    let hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1259101)
                && event.source == Some(caster.to_bits())
        })
        .cloned()
        .collect();
    assert!(
        !hits.is_empty(),
        "native Dummy root must cast its actual1259101 damage child"
    );
    assert!(hits.iter().all(|event| event.crit));
    h.advance(8);
    assert!(aura_now(h.app.world()) - begin < 0.5);
    assert!(
        targets
            .iter()
            .all(|target| !crate::auras::has_aura(h.app.world(), *target, 188389)),
        "TC's500ms Flame Shock callback must not execute early"
    );
    h.advance(2);
    for hit in hits {
        let target = Entity::try_from_bits(hit.target.unwrap()).unwrap();
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == 188389 && aura.caster == caster)),
            "actual Voltaic hit must receive its delayed native Flame Shock"
        );
    }
}
