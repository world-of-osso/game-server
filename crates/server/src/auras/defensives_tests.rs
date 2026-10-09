//! Catalog-backed defensive casts followed by production spell damage and melee.
//! No defensive aura is injected, and real DB2 spells/effects remain unchanged.

use crate::auras::AuraStatMods;
use crate::item_stats::WeaponDamage;
use crate::stats::CharacterStats;
use shared::components::WeaponType;
use shared::protocol::{CombatLogKind, MissKind};

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

fn incoming_melee(h: &mut Harness) -> f32 {
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
    let before = h.health(h.caster);
    h.advance(1);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<crate::combat::AutoAttack>();
    before - h.health(h.caster)
}

fn assert_reduction(class: u8, spell: u32, triggered: &[u32], expected: f32) {
    let mut h = cast_defensive(class, spell, triggered);
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), expected);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(FIREBALL)
            && event.kind == CombatLogKind::Damage
            && event.amount == expected as i32
    }));
    assert_eq!(incoming_melee(&mut h), expected);
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
#[ignore = "defensive gap: DUMMY effects have no Fortifying Brew script (spell_cast/scripts.rs:36-45)"]
fn defensives_monk_fortifying_brew_reduces_fire_20_percent() {
    assert_reduction(10, 115_203, &[], 80.0);
}

#[test]
#[ignore = "defensive gap: aura 49 unhandled; player_defense reads rating only (auras/handlers.rs:197, stats.rs:430)"]
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
    // Evasion does not mitigate fire; its catalog effect is +200 dodge percent.
    assert_eq!(incoming_fire(&mut h, INCOMING_DAMAGE), INCOMING_DAMAGE);
    let after = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .defense
        .dodge;
    assert_eq!(after - before, 200.0);
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
    assert_eq!(incoming_melee(&mut h), INCOMING_DAMAGE);
}

#[test]
#[ignore = "defensive gap: melee applies health loss without school immunity (combat.rs:844-853)"]
fn defensives_paladin_divine_shield_blocks_melee() {
    let mut h = cast_defensive(2, 642, &[]);
    assert_eq!(incoming_melee(&mut h), 0.0);
}

#[test]
#[ignore = "defensive gap: melee applies health loss without school immunity (combat.rs:844-853)"]
fn defensives_mage_ice_block_blocks_melee() {
    let mut h = cast_defensive(8, 45_438, &[]);
    assert_eq!(incoming_melee(&mut h), 0.0);
}

#[test]
#[ignore = "defensive gap: melee applies health loss without absorb_damage (combat.rs:844-853)"]
fn defensives_priest_power_word_shield_absorbs_melee() {
    let mut h = cast_defensive(5, 17, &[]);
    let before = shield_remaining(&h);
    assert!(before > INCOMING_DAMAGE);
    assert_eq!(incoming_melee(&mut h), 0.0);
    assert_eq!(shield_remaining(&h), before - INCOMING_DAMAGE);
}
