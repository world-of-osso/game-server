//! Catalog-backed defensive casts followed by production spell damage and melee.
//! No defensive aura is injected, and real DB2 spells/effects remain unchanged.

use crate::auras::AuraStatMods;
use crate::item_stats::WeaponDamage;
use crate::stats::CharacterStats;
use shared::components::WeaponType;
use shared::protocol::{CombatEventType, CombatLogKind, MissKind};

use crate::auras::ActiveAuras;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;

const FIREBALL: u32 = 133;
const INCOMING_DAMAGE: f32 = 100.0;

fn cast_defensive(class: u8, spell: u32, triggered: &[u32]) -> Harness {
    let mut ids = vec![spell, FIREBALL];
    ids.extend_from_slice(triggered);
    let mut h = harness(Vec::new(), &ids, class).expect("real world.db required");
    h.cast_and_deliver(spell, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h
}

/// Feed a fixed pre-mitigation amount to the same production handler used by
/// SCHOOL_DAMAGE and periodic damage. No random hit/crit/variance roll.
fn incoming_fire(h: &mut Harness, amount: f32) -> f32 {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .clone();
    let before = h.health(h.caster);
    let ctx = EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(h.app.world_mut(), &ctx, h.caster, amount, false, false);
    let loss = before - h.health(h.caster);
    h.advance(3);
    loss
}

struct MeleeImpact {
    health_loss: f32,
    critical_multiplier: f32,
}

fn start_npc_swing(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.npc).insert((
        CharacterStats {
            main_hand: WeaponDamage {
                min: INCOMING_DAMAGE,
                max: INCOMING_DAMAGE,
                speed: 2.6,
                kind: WeaponType::OneHand,
            },
            ..Default::default()
        },
        AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..Default::default()
        },
        crate::combat::AutoAttack::new(h.caster),
    ));
}

fn incoming_melee(h: &mut Harness) -> MeleeImpact {
    start_npc_swing(h);
    let before = h.health(h.caster);
    h.advance(1);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<crate::combat::AutoAttack>();
    let health_loss = before - h.health(h.caster);
    h.advance(3);
    let event = h
        .inbox
        .log
        .iter()
        .rev()
        .find(|event| {
            event.spell_id.is_none()
                && event.source == Some(h.npc.to_bits())
                && event.target == Some(h.caster.to_bits())
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .expect("delivered melee combat log");
    // NPCs retain their 5% crit chance even with CharacterStats::crit_pct = 0.
    // Validate both actual hit outcomes rather than changing NPC gameplay.
    MeleeImpact {
        health_loss,
        critical_multiplier: if event.crit { 2.0 } else { 1.0 },
    }
}

fn assert_melee_event(h: &Harness, kind: CombatEventType, amount: f32) {
    let log = h.app.world().resource::<crate::combat::CombatLog>();
    let entries = log.range(0.0, f64::MAX);
    let event = &entries.last().expect("scheduled swing event").event;
    assert_eq!(event.event_type, kind);
    assert_eq!(event.amount, amount);
}

fn assert_reduction(class: u8, spell: u32, triggered: &[u32], expected: f32) {
    let mut h = cast_defensive(class, spell, triggered);
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), expected);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(FIREBALL)
            && event.kind == CombatLogKind::Damage
            && event.amount == expected as i32
    }));
    let melee = incoming_melee(&mut h);
    assert_eq!(melee.health_loss, expected * melee.critical_multiplier);
}

fn assert_immunity(class: u8, spell: u32) {
    let mut h = cast_defensive(class, spell, &[]);
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), 0.0);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(FIREBALL) && event.kind == CombatLogKind::Miss(MissKind::Immune)
    }));
}

#[test]
fn defensives_paladin_divine_shield_blocks_fire() {
    assert_immunity(2, 642);
}

#[test]
fn defensives_mage_ice_block_blocks_fire() {
    assert_immunity(8, 45_438);
}

#[test]
fn defensives_warrior_shield_wall_reduces_fire_40_percent() {
    assert_reduction(1, 871, &[], 60.0);
}

#[test]
fn defensives_hunter_turtle_reduces_fire_30_percent() {
    assert_reduction(3, 186_265, &[], 70.0);
}

#[test]
fn defensives_warlock_unending_resolve_reduces_fire_25_percent() {
    assert_reduction(9, 104_773, &[], 75.0);
}

#[test]
fn defensives_druid_barkskin_reduces_fire_20_percent() {
    assert_reduction(11, 22_812, &[], 80.0);
}

#[test]
fn defensives_shaman_astral_shift_reduces_fire_40_percent() {
    assert_reduction(7, 108_271, &[], 60.0);
}

#[test]
fn defensives_death_knight_icebound_fortitude_reduces_fire_30_percent() {
    assert_reduction(6, 48_792, &[], 70.0);
}

#[test]
fn defensives_evoker_obsidian_scales_reduces_fire_30_percent() {
    assert_reduction(13, 363_916, &[], 70.0);
}

#[test]
fn defensives_demon_hunter_blur_trigger_reduces_fire_25_percent() {
    assert_reduction(12, 198_589, &[212_800], 75.0);
}

#[test]
fn defensives_monk_fortifying_brew_reduces_fire_20_percent() {
    let mut h =
        harness(Vec::new(), &[115_203, 120_954, FIREBALL], 10).expect("real world.db required");
    // A wounded unit gains the same fixed bonus to current and max health.
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .current = 700.0;
    h.cast_and_deliver(115_203, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), 80.0);
    let melee = incoming_melee(&mut h);
    let expected_melee = 80.0 * melee.critical_multiplier;
    assert_eq!(melee.health_loss, expected_melee);
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(health.max, 1200.0);
    assert_eq!(health.current, 820.0 - expected_melee);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 120_954,
    );
    h.advance(3);
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(health.max, 1000.0);
    assert_eq!(health.current, 620.0 - expected_melee);
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), INCOMING_DAMAGE);
}

#[test]
fn defensives_monk_brew_health_survives_stat_updates() {
    let mut h = crate::spell_cast::test_harness::harness_with(
        Vec::new(),
        &[115_203, 120_954, FIREBALL],
        10,
        true,
    )
    .expect("real world.db and computed stats required");
    h.advance(3);
    let base_max = h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap()
        .max;
    let wounded = (base_max * 0.7).floor();
    let bonus = (base_max * 0.2).floor();
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .current = wounded;
    h.cast_and_deliver(115_203, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(3);
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(health.max, base_max + bonus);
    assert_eq!(health.current, wounded + bonus);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 120_954,
    );
    h.advance(3);
    let health = h
        .app
        .world()
        .get::<shared::components::Health>(h.caster)
        .unwrap();
    assert_eq!(health.max, base_max);
    assert_eq!(health.current, wounded);
}

#[test]
fn defensives_rogue_evasion_increases_dodge() {
    let mut h =
        crate::spell_cast::test_harness::harness_with(Vec::new(), &[5277, FIREBALL], 4, true)
            .expect("real world.db and computed stats required");
    h.advance(3);
    let before = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .defense
        .dodge;
    h.cast_and_deliver(5277, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    // TrinityCore adds aura 49 as nondiminishing percentage points. This catalog
    // has 200 with no scaling/variance or per-level term, not a 2x encoding.
    // Both 100 and 200 exhaust the melee hit table; fire is not mitigated.
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), INCOMING_DAMAGE);
    let after = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .defense
        .dodge;
    assert_eq!(after - before, 200.0);
    assert_eq!(incoming_melee(&mut h).health_loss, 0.0);
    h.advance(3);
    assert!(h.inbox.log.iter().any(
        |event| event.spell_id.is_none() && event.kind == CombatLogKind::Miss(MissKind::Dodge)
    ));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 5277,
    );
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .defense
            .dodge,
        before
    );
}

fn shield_remaining(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .map(|auras| {
            auras
                .iter()
                .filter(|aura| aura.spell_id == 17)
                .flat_map(|aura| &aura.effects)
                .map(|effect| effect.absorb_remaining)
                .sum()
        })
        .unwrap_or(0.0)
}

#[test]
fn defensives_priest_power_word_shield_absorbs_and_is_consumed() {
    let mut h = cast_defensive(5, 17, &[]);
    let shield = shield_remaining(&h);
    assert!(shield > INCOMING_DAMAGE, "shield capacity {shield}");
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), 0.0);
    assert_eq!(shield_remaining(&h), shield - INCOMING_DAMAGE);
    // Consume the rest and pass exactly 100 damage through to health.
    assert_eq!(incoming_fire(&mut h, shield), INCOMING_DAMAGE);
    assert_eq!(shield_remaining(&h), 0.0);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 17));
}

#[test]
fn defensives_unprotected_control_takes_full_fire_damage() {
    let mut h = harness(Vec::new(), &[FIREBALL], 1).expect("real world.db required");
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), INCOMING_DAMAGE);
    let melee = incoming_melee(&mut h);
    assert_eq!(
        melee.health_loss,
        INCOMING_DAMAGE * melee.critical_multiplier
    );
}

#[test]
fn defensives_paladin_divine_shield_blocks_melee() {
    let mut h = cast_defensive(2, 642, &[]);
    assert_eq!(incoming_melee(&mut h).health_loss, 0.0);
    assert_melee_event(&h, CombatEventType::Miss, 0.0);
    h.advance(3);
    assert!(h.inbox.log.iter().any(
        |event| event.spell_id.is_none() && event.kind == CombatLogKind::Miss(MissKind::Immune)
    ));
}

#[test]
fn defensives_mage_ice_block_blocks_melee() {
    let mut h = cast_defensive(8, 45_438, &[]);
    assert_eq!(incoming_melee(&mut h).health_loss, 0.0);
    assert_melee_event(&h, CombatEventType::Miss, 0.0);
    h.advance(3);
    assert!(h.inbox.log.iter().any(
        |event| event.spell_id.is_none() && event.kind == CombatLogKind::Miss(MissKind::Immune)
    ));
}

#[test]
fn defensives_priest_power_word_shield_absorbs_melee() {
    let mut h = cast_defensive(5, 17, &[]);
    let before = shield_remaining(&h);
    assert!(before > INCOMING_DAMAGE * 2.0);
    let melee = incoming_melee(&mut h);
    let incoming = INCOMING_DAMAGE * melee.critical_multiplier;
    assert_eq!(melee.health_loss, 0.0);
    assert_eq!(shield_remaining(&h), before - incoming);
    assert_melee_event(&h, CombatEventType::Absorb, incoming);
    h.advance(3);
    assert!(h.inbox.log.iter().any(|event| event.spell_id.is_none()
        && event.kind == CombatLogKind::Miss(MissKind::Absorb)
        && event.amount == 0
        && event.absorbed == incoming as i32));
    // Capacity is fractional (catalog coefficient 4.638), while spell damage
    // rounds to whole points. Measure the remainder instead of assuming 50.
    incoming_fire(&mut h, before - incoming - 50.0);
    let remaining = shield_remaining(&h);
    assert!(remaining > 0.0 && remaining < INCOMING_DAMAGE);
    let melee = incoming_melee(&mut h);
    let expected_damage = (INCOMING_DAMAGE * melee.critical_multiplier - remaining).round();
    assert_eq!(melee.health_loss, expected_damage);
    assert_eq!(shield_remaining(&h), 0.0);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 17));
    h.advance(3);
    assert!(
        h.inbox.log.iter().any(|event| event.spell_id.is_none()
            && event.kind == CombatLogKind::Damage
            && event.amount == expected_damage as i32
            && event.absorbed == remaining as i32),
        "partial melee logs: {:?}",
        h.inbox
            .log
            .iter()
            .filter(|event| event.spell_id.is_none())
            .collect::<Vec<_>>()
    );
}
