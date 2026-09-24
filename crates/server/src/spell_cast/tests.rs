//! Bevy App tests with real SpellInfo from data/world.db (skipped when absent).

use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{CombatStatus, Health, Mana, Position, PowerType, UnitPowers};
use shared::protocol::{CastFailed, CombatLogKind, SpellCooldownUpdate};
use shared::spell_data::CastFailReason;

use crate::class_progression::KnownSpells;
use crate::networking::Moving;
use crate::spell_info::{SpellEffect, SpellInfo};
use crate::stats::CharacterStats;

use super::test_harness::*;

const MORTAL_STRIKE: u32 = 12294;
const FIREBALL: u32 = 133;
const KICK: u32 = 1766;
const FLASH_HEAL: u32 = 2061;
const CHARGE: u32 = 100;

#[test]
fn warrior_gets_rage_pool_replicated_as_unit_powers() {
    let Some(mut h) = harness(Vec::new(), &[], 1) else {
        return;
    };
    h.advance(1);
    let powers = h.app.world().get::<UnitPowers>(h.caster).unwrap();
    assert_eq!(powers.entries.len(), 1);
    assert_eq!(powers.entries[0].power, PowerType::Rage);
    assert_eq!(
        (powers.entries[0].current, powers.entries[0].max),
        (0, 1000)
    );
}

#[test]
fn instant_spell_resolves_same_tick_spending_cost_and_starting_gcd_and_cooldown() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    // AP coefficient 5.2012 × attack power 100 = 520.
    assert_eq!(h.health(h.npc), 5000.0 - 520.0);
    assert_eq!(h.power(PowerType::Rage), 200.0, "30 rage = 300 raw spent");
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.advance(2);
    let mut cooldowns = std::mem::take(&mut h.inbox.cooldowns);
    cooldowns.sort_by_key(|update| update.is_gcd);
    assert_eq!(
        cooldowns,
        vec![
            SpellCooldownUpdate {
                spell_id: MORTAL_STRIKE,
                category: 0,
                duration_ms: 6000,
                remaining_ms: 6000,
                is_gcd: false,
            },
            SpellCooldownUpdate {
                spell_id: MORTAL_STRIKE,
                category: 133,
                duration_ms: 1500,
                remaining_ms: 1500,
                is_gcd: true,
            },
        ]
    );
    let log = std::mem::take(&mut h.inbox.log);
    assert!(log.iter().any(|e| e.kind == CombatLogKind::CastSuccess));
    let damage = log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage)
        .unwrap();
    assert_eq!((damage.amount, damage.overflow), (520, 0));

    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::OnGlobalCooldown);
    h.advance(30);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed)[0].reason,
        CastFailReason::OnCooldown
    );
    assert_eq!(h.health(h.npc), 5000.0 - 520.0);
}

#[test]
fn cast_time_spell_completes_after_its_cast_time() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    let cast = h.app.world().get::<CastState>(h.caster).expect("cast bar");
    assert_eq!(cast.duration, 1.75);
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10_000.0
    );
    h.advance(30);
    assert!(h.app.world().get::<CastState>(h.caster).unwrap().elapsed < 1.75);
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(10);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    // SP coefficient 3.8812 × spell power 100 = 388.
    assert_eq!(h.health(h.npc), 5000.0 - 388.0);
    // 1% of base mana (100) spent on completion, then at most 0.25 s of 1%/s regen.
    let mana = h.app.world().get::<Mana>(h.caster).unwrap().current;
    assert!((9_900.0..=9_925.0).contains(&mana), "mana {mana}");
    let kinds: Vec<_> = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            CombatLogKind::CastStart,
            CombatLogKind::CastSuccess,
            CombatLogKind::Damage
        ]
    );
}

#[test]
fn moving_cancels_cast_time_spell_without_effects() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.app.world_mut().get_mut::<Moving>(h.caster).unwrap().0 = true;
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    h.advance(40);
    assert_eq!(h.health(h.npc), 5000.0);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed)[0].reason,
        CastFailReason::CantCastWhileMoving
    );
}

#[test]
fn kick_interrupts_target_cast_and_locks_its_school() {
    let Some(mut h) = harness(Vec::new(), &[KICK, FIREBALL], 4) else {
        return;
    };
    // NPCs do not cast yet; give the NPC a Fireball cast bar directly.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CastState::normal(FIREBALL, 0, 1.75, true));
    h.cast_and_deliver(KICK, Some(h.npc));
    assert!(
        h.app.world().get::<CastState>(h.npc).is_none(),
        "cast interrupted"
    );
    let history = h.app.world().get::<super::SpellHistory>(h.npc).unwrap();
    let now = h.app.world().resource::<Time>().elapsed_secs_f64();
    let fire = 4;
    assert!(history.school_locked(fire, now + 5.5), "6 s Fire lockout");
    assert!(!history.school_locked(fire, now + 6.1));
    assert!(!history.school_locked(1, now), "Physical not locked");
    h.advance(2);
    let log = std::mem::take(&mut h.inbox.log);
    let interrupt = log
        .iter()
        .find(|e| e.kind == CombatLogKind::Interrupt)
        .unwrap();
    assert_eq!(interrupt.spell_id, Some(KICK));
    assert_eq!(interrupt.school_mask, fire);
}

#[test]
fn out_of_range_sends_cast_failed_to_owner() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 30.0;
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed),
        vec![CastFailed {
            spell_id: MORTAL_STRIKE,
            reason: CastFailReason::OutOfRange,
            detail: None,
        }]
    );
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.power(PowerType::Rage), 500.0);
}

#[test]
fn heal_clamps_to_max_and_logs_overheal() {
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL], 5) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 900.0;
    // No explicit ally target: auto self-cast.
    h.cast_and_deliver(FLASH_HEAL, None);
    h.advance(32);
    assert_eq!(h.health(h.caster), 1000.0);
    let heal = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Heal)
        .unwrap();
    // SP coefficient 6.82 × spell power 100 = 682; 100 effective.
    assert_eq!((heal.amount, heal.overflow), (682, 582));
}

#[test]
fn friendly_heal_on_hostile_npc_self_casts_instead() {
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL], 5) else {
        return;
    };
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 1000.0;
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 900.0;
    h.cast_and_deliver(FLASH_HEAL, Some(h.npc));
    h.advance(32);
    assert_eq!(h.health(h.npc), 1000.0, "hostile kobold is not healed");
    assert_eq!(h.health(h.caster), 1000.0);
    let heal = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Heal)
        .unwrap();
    assert_eq!(heal.target, Some(h.caster.to_bits()));
}

#[test]
fn energize_adds_raw_units() {
    let Some(mut h) = harness(Vec::new(), &[CHARGE], 1) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    h.cast_and_deliver(CHARGE, Some(h.npc));
    assert_eq!(h.power(PowerType::Rage), 200.0, "Charge: 20 rage = 200 raw");
    h.advance(2);
    let energize = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Energize)
        .unwrap();
    assert_eq!((energize.amount, energize.overflow), (200, 0));
    let charges = std::mem::take(&mut h.inbox.charges);
    assert_eq!(
        (charges[0].current, charges[0].max, charges[0].recharge_ms),
        (0, 1, 20_000)
    );
}

#[test]
fn rage_decays_out_of_combat_only() {
    let Some(mut h) = harness(Vec::new(), &[], 1) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    h.set_power(PowerType::Rage, 500.0);
    h.advance(20);
    assert_eq!(h.power(PowerType::Rage), 500.0, "no decay in combat");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.advance(40);
    // RegenPeace -12.5 raw/s over 2 s.
    let rage = h.power(PowerType::Rage);
    assert!((474.0..=476.0).contains(&rage), "rage {rage}");
    let powers = h.app.world().get::<UnitPowers>(h.caster).unwrap();
    assert_eq!(powers.entries[0].current, rage.floor() as i32);
}

/// Synthetic chain 900001 → 900002 → … each energizing 10 raw rage on the
/// caster and triggering the next.
fn trigger_chain(length: u32) -> Vec<SpellInfo> {
    (0..length)
        .map(|step| SpellInfo {
            id: 900_001 + step,
            name: format!("Chain {step}"),
            effects: vec![
                SpellEffect {
                    index: 0,
                    effect: super::effects::EFFECT_ENERGIZE,
                    base_points: 10.0,
                    misc_values: [1, 0],
                    implicit_targets: [1, 0],
                    ..default()
                },
                SpellEffect {
                    index: 1,
                    effect: super::effects::EFFECT_TRIGGER_SPELL,
                    trigger_spell: 900_002 + step,
                    implicit_targets: [1, 0],
                    ..default()
                },
            ],
            ..default()
        })
        .collect()
}

#[test]
fn triggered_spell_chain_is_bounded() {
    let Some(mut h) = harness(trigger_chain(8), &[], 1) else {
        return;
    };
    h.cast_and_deliver(900_001, None);
    // The requested spell plus MAX_TRIGGER_DEPTH (3) triggered hops.
    assert_eq!(h.power(PowerType::Rage), 40.0);
}

#[test]
fn unsupported_implicit_target_fails_the_cast() {
    let mut spell = trigger_chain(1).remove(0);
    spell.effects[0].implicit_targets = [87, 16];
    let Some(mut h) = harness(vec![spell], &[], 1) else {
        return;
    };
    h.cast_and_deliver(900_001, None);
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::NotSupported);
    assert_eq!(
        failed[0].detail.as_deref(),
        Some("implicit target 87 not supported")
    );
    assert_eq!(h.power(PowerType::Rage), 0.0);
}

#[test]
fn unknown_spell_is_rejected_as_not_known() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(Vec::new()));
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed),
        vec![CastFailed {
            spell_id: MORTAL_STRIKE,
            reason: CastFailReason::NotSupported,
            detail: Some("spell not known".into()),
        }]
    );
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.power(PowerType::Rage), 500.0);
}

#[test]
fn computed_stats_make_fireball_scale_with_intellect() {
    let Some(mut h) = harness_with(Vec::new(), &[FIREBALL], 8, true) else {
        return;
    };
    // Level 10 Human Mage: base Int 38 = spell power 38; gtBaseMP 1000;
    // Sta 555 × gtHpPerSta 2 = 1110 health (was 100/100: stays full).
    let stats = *h.app.world().get::<CharacterStats>(h.caster).unwrap();
    assert_eq!((stats.spell_power, stats.base_mana), (38.0, 1000.0));
    assert_eq!(
        *h.app.world().get::<Health>(h.caster).unwrap(),
        Health {
            current: 1110.0,
            max: 1110.0
        }
    );
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(40);
    let damage = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Damage)
        .expect("fireball hit");
    // SP coefficient 3.8812 × 38 = 147 (×2 on a 5% crit).
    let expected = if damage.crit { 295 } else { 147 };
    assert_eq!(damage.amount, expected);
}

#[test]
fn percent_mana_cost_uses_base_mana_not_max_pool() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .base_mana = 2_000.0;
    // Cost is 1% of base mana 2000 = 20; 1% of the 10 000 max pool would be
    // 100. 40 mana plus 0.15 s of 1%/s base mana regen (3) affords only the
    // former.
    h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 40.0;
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(std::mem::take(&mut h.inbox.failed).is_empty());
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.advance(40);
    assert_eq!(h.health(h.npc), 5000.0 - 388.0);
}

/// Level 80 caster knowing the real class/spec passives `passives`, 50 000
/// gtBaseMP (all mana classes at 80) and an empty 250 000 pool (regen
/// follows base mana, not the pool). Returns the
/// mana gained over 1 s in combat, then over 1 s out of combat.
fn mana_regen_per_second(class: u8, passives: &[u32]) -> Option<(f32, f32)> {
    let mut h = harness(Vec::new(), passives, class)?;
    let base_mana = crate::stats::base_mana(class, 80).unwrap();
    assert_eq!(base_mana, 50_000.0);
    let world = h.app.world_mut();
    world
        .entity_mut(h.caster)
        .insert(shared::components::Level(80));
    world.get_mut::<CharacterStats>(h.caster).unwrap().base_mana = base_mana;
    // Let the passives apply before measuring.
    h.advance(2);
    let measure = |h: &mut Harness| {
        *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
            current: 0.0,
            max: 250_000.0,
        };
        h.advance(20);
        h.app.world().get::<Mana>(h.caster).unwrap().current
    };
    let in_combat = measure(&mut h);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    let out_of_combat = measure(&mut h);
    Some((in_combat, out_of_combat))
}

fn assert_near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.5, "{actual} != {expected}");
}

#[test]
fn holy_paladin_regenerates_one_percent_base_mana_scaled_by_class_and_spec_passives() {
    // Paladin 137026: MOD_MANA_REGEN_PCT -36; Holy Paladin 1258016: +25.
    // 1% of 50 000 = 500/s × 0.64 × 1.25 = 400/s, in and out of combat.
    let Some((in_combat, out_of_combat)) = mana_regen_per_second(2, &[137026, 137029, 1258016])
    else {
        return;
    };
    assert_near(in_combat, 400.0);
    assert_near(out_of_combat, 400.0);
}

#[test]
fn frost_mage_regenerates_one_percent_base_mana_per_second() {
    // Mage 137018 and Frost Mage 137020/1256913 carry no regen modifier.
    let Some((in_combat, out_of_combat)) = mana_regen_per_second(8, &[137018, 137020, 1256913])
    else {
        return;
    };
    assert_near(in_combat, 500.0);
    assert_near(out_of_combat, 500.0);
}

#[test]
fn arcane_mage_power_regen_aura_raises_mana_regen_by_its_percentage() {
    // Arcane Mage 137021: MOD_POWER_REGEN_PERCENT +60 on mana: 500 × 1.6.
    let Some((in_combat, out_of_combat)) = mana_regen_per_second(8, &[137018, 137021]) else {
        return;
    };
    assert_near(in_combat, 800.0);
    assert_near(out_of_combat, 800.0);
}
