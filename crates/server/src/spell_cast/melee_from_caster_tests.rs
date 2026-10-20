//! TC a352b1fa Unit.cpp8213–8219: MOD_MELEE_DAMAGE_FROM_CASTER343 multiplies
//! white swings (no spell) from the aura's caster. Colossus Smash167105 applies
//! native208086 effects0/1 to its hit target (spell_warrior.cpp583–598).
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::melee::{Hand, weapon_damage_range};
use crate::stats::CharacterStats;
use bevy::prelude::Entity;
use shared::protocol::CombatLogKind;
use std::ops::RangeInclusive;

const COLOSSUS_SMASH: u32 = 167105;
const COLOSSUS_WOUND: u32 = 208086;
const MELEE_DAMAGE_FROM_CASTER: u32 = 343;

fn warrior() -> Harness {
    harness_with_native_enemy_dodge_reduction(Vec::new(), &[COLOSSUS_SMASH, COLOSSUS_WOUND], 1)
        .unwrap()
}

/// The owner's whole-number main-hand roll (`melee::roll_weapon_damage`); the
/// armorless fixture kobold applies no mitigation.
fn native_swing_range(h: &Harness) -> RangeInclusive<f32> {
    let stats = h.app.world().get::<CharacterStats>(h.caster).unwrap();
    let (min, max) = weapon_damage_range(
        &stats.main_hand,
        stats.attack_power,
        stats.versatility_pct,
        Hand::Main,
    );
    min.trunc()..=max.trunc()
}

fn scaled(range: &RangeInclusive<f32>, multiplier: f32) -> RangeInclusive<f32> {
    (range.start() * multiplier).round()..=(range.end() * multiplier).round()
}

/// First unblocked, non-critical, non-glancing owner white swing within `frames`.
fn normal_swing(h: &mut Harness, frames: usize) -> Option<f32> {
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 0.0;
    let start = h.inbox.log.len();
    let (owner, npc) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::combat::AutoAttack::new(npc));
    let mut damage = None;
    for _ in 0..frames {
        h.advance(1);
        damage = h.inbox.log[start..]
            .iter()
            .find(|event| {
                event.source == Some(owner.to_bits())
                    && event.target == Some(npc.to_bits())
                    && event.spell_id.is_none()
                    && event.kind == CombatLogKind::Damage
                    && !event.crit
                    && !event.glancing
                    && event.blocked == 0
            })
            .map(|event| event.amount as f32);
        if damage.is_some() {
            break;
        }
    }
    h.app
        .world_mut()
        .entity_mut(owner)
        .remove::<crate::combat::AutoAttack>();
    damage
}

fn wound_while_active(h: &mut Harness) -> f32 {
    // Native10s wound; P(no Normal swing in ~5 attempts) is about1.4e-4.
    let damage = normal_swing(h, 190).expect("no Normal white swing inside native208086");
    assert!(
        crate::auras::has_aura(h.app.world(), h.npc, COLOSSUS_WOUND),
        "swing must land while208086 is active"
    );
    damage
}

fn cast_colossus_smash(h: &mut Harness) {
    let npc = h.npc;
    h.cast_and_deliver(COLOSSUS_SMASH, Some(npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), npc, COLOSSUS_WOUND));
}

#[test]
fn class_completeness_native_colossus_smash_applies_owner_melee_vulnerability_343() {
    let mut h = warrior();
    cast_colossus_smash(&mut h);
    let auras = h.app.world().get::<ActiveAuras>(h.npc).unwrap();
    let wound = auras
        .iter()
        .find(|aura| aura.spell_id == COLOSSUS_WOUND)
        .unwrap();
    assert_eq!(wound.caster, h.caster);
    let melee: Vec<f32> = wound
        .effects
        .iter()
        .filter(|effect| effect.aura_type == MELEE_DAMAGE_FROM_CASTER)
        .map(|effect| effect.amount)
        .collect();
    assert_eq!(melee, vec![30.0], "native208086 effect1 aura343 amount30");
}

#[test]
fn class_completeness_native_colossus_smash_amplifies_only_owner_white_swings() {
    let mut h = warrior();
    let base = native_swing_range(&h);
    let unbuffed = normal_swing(&mut h, 360).expect("no Normal white swing in18s");
    assert!(
        base.contains(&unbuffed),
        "fixture swing {unbuffed} outside {base:?}"
    );
    cast_colossus_smash(&mut h);
    let wounded = wound_while_active(&mut h);
    let amplified = scaled(&base, 1.30);
    assert!(
        amplified.contains(&wounded),
        "aura343 +30%: white swing {wounded} outside {amplified:?} (base {base:?})"
    );
    for _ in 0..240 {
        if !crate::auras::has_aura(h.app.world(), h.npc, COLOSSUS_WOUND) {
            break;
        }
        h.advance(1);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        COLOSSUS_WOUND
    ));
    let expired = normal_swing(&mut h, 360).expect("no Normal white swing in18s");
    assert!(
        base.contains(&expired),
        "expiry restores {base:?}, got {expired}"
    );
}

#[test]
fn class_completeness_native_foreign_melee_vulnerability_343_ignores_owner_swings() {
    let mut h = warrior();
    let base = native_swing_range(&h);
    let foreign: Entity = spawn_player(&mut h.app, 1, 0.0);
    let npc = h.npc;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: foreign,
            target: npc,
            spell_id: COLOSSUS_WOUND,
            effect_mask: 0b11,
            source: AuraSource::Cast,
        },
    );
    let swing = wound_while_active(&mut h);
    assert!(
        base.contains(&swing),
        "foreign343 changed owner swing {swing}; {base:?}"
    );
}
