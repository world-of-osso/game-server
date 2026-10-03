//! Rage from auto-attacks dealt and taken, Retail values from SimC
//! `sc_warrior.cpp` (`melee_t::trigger_rage_gain`, `warrior_t::assess_damage`).

use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::components::PowerType;

/// Arms Warrior spec passive: effect 6 MOD_RAGE_FROM_DAMAGE_DEALT +250%.
const ARMS_WARRIOR: u32 = 137049;

fn warrior(known: &[u32]) -> Option<Harness> {
    let mut h = harness(Vec::new(), known, 1)?;
    h.advance(1);
    h.set_power(PowerType::Rage, 0.0);
    Some(h)
}

/// The caster attacks through `CMSG_ATTACK_SWING`, arriving 3 frames later; the
/// NPC through its AI target, in the next frame. The NPC gets MOD_HIT_CHANCE 100:
/// a creature's swing otherwise misses 5% of the time (`UNIT_MISS_CHANCE_PCT`,
/// Unit::MeleeSpellMissChance) and a miss does no damage, so gives no rage.
fn attack(h: &mut Harness, attacker: Entity, target: Entity) {
    if attacker == h.caster {
        h.attack(target);
    } else {
        h.app.world_mut().entity_mut(attacker).insert((
            CombatTarget(Some(target)),
            crate::auras::AuraStatMods {
                melee_hit_chance_pct: 100.0,
                ..Default::default()
            },
        ));
    }
}

#[test]
fn each_swing_dealt_gives_swing_time_times_1_75_rage() {
    let Some(mut h) = warrior(&[]) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    attack(&mut h, caster, npc);
    // Swings in the frame the request arrives and 2 s later: 2.0 s × 1.75 = 3.5 rage (35 raw) each.
    h.advance(3);
    assert_eq!(h.power(PowerType::Rage), 35.0);
    h.advance(43);
    assert_eq!(h.power(PowerType::Rage), 70.0);
}

#[test]
fn rage_from_damage_dealt_auras_scale_swing_rage() {
    let Some(mut h) = warrior(&[ARMS_WARRIOR]) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    attack(&mut h, caster, npc);
    h.advance(3);
    // 3.5 rage × (1 + 250%) = 12.25, rounded to 12.3 rage.
    assert_eq!(h.power(PowerType::Rage), 123.0);
}

#[test]
fn each_auto_attack_taken_gives_3_rage() {
    let Some(mut h) = warrior(&[]) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    attack(&mut h, npc, caster);
    h.advance(2);
    assert_eq!(h.power(PowerType::Rage), 30.0);
    h.advance(43);
    assert_eq!(h.power(PowerType::Rage), 60.0);
}

#[test]
fn rage_from_hits_taken_has_a_one_second_cooldown() {
    let Some(mut h) = warrior(&[]) else {
        return;
    };
    let caster = h.caster;
    let second = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 2.0);
    let first = h.npc;
    attack(&mut h, first, caster);
    attack(&mut h, second, caster);
    // Both kobolds hit in the same tick: only the first gives rage.
    h.advance(2);
    assert_eq!(h.power(PowerType::Rage), 30.0);
}

#[test]
fn a_rage_pool_that_is_not_the_primary_power_gains_nothing() {
    // Druid (class 11): mana primary, rage pool for Bear Form
    // (AzerothCore `HasActivePowerType(POWER_RAGE)`).
    let Some(mut h) = harness(Vec::new(), &[], 11) else {
        return;
    };
    h.advance(1);
    h.set_power(PowerType::Rage, 0.0);
    let (caster, npc) = (h.caster, h.npc);
    attack(&mut h, caster, npc);
    attack(&mut h, npc, caster);
    h.advance(3);
    assert_eq!(h.power(PowerType::Rage), 0.0);
}
