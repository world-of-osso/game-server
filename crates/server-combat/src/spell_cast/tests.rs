//! Bevy App tests with real SpellInfo from data/world.db.

use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{CombatStatus, Health, Mana, Position, PowerType, UnitPowers};
use shared::protocol::{CastFailed, CombatLogKind, SpellCooldownUpdate, SpellFailure, SpellGo};
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
const FROSTBOLT: u32 = 116;
/// Frostbolt's TRIGGER_MISSILE (32) spell: SCHOOL_DAMAGE 2.448 × SP, Variance 0.05.
const FROSTBOLT_DAMAGE: u32 = 228597;
const CHARGE: u32 = 100;
const SLAM: u32 = 1464;
const BATTLE_SHOUT: u32 = 6673;

#[test]
fn arena_exclude_caster_filters_source_36003_before_its_one_target_cap() {
    let mut h = harness(Vec::new(), &[36003], 8).unwrap();
    let ally = spawn_player(&mut h.app, 8, 3.0);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(36003)
        .unwrap()
        .clone();
    let cap = spell.target_restrictions.as_ref().unwrap().max_targets;
    assert_eq!(cap, 1);
    let effect = &spell.effects[0];
    assert_eq!(super::targets::effect_radius(effect), 25.0);
    let selected = super::targets::select_targets(
        h.app.world_mut(),
        h.caster,
        None,
        &spell,
        effect,
        cap,
        false,
    )
    .unwrap();
    assert_eq!(
        selected,
        vec![ally],
        "caster must not consume the eligible ally's slot"
    );
}

#[test]
fn arena_exclude_caster_rejects_explicit_self_before_cast_effects() {
    let mut h = harness(Vec::new(), &[36003], 8).unwrap();
    h.cast_and_deliver(36003, Some(h.caster));
    assert_eq!(
        h.inbox.failed,
        vec![CastFailed {
            spell_id: 36003,
            reason: CastFailReason::InvalidTarget,
            detail: None,
        }]
    );
    assert!(h.inbox.go.is_empty());
}

#[test]
fn arena_preparation_school_mask_power_cost_and_unit_flag_follow_actual_aura() {
    let mut h = harness(Vec::new(), &[32727, FIREBALL], 8).unwrap();
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .base_mana = 2_000.0;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .clone();
    let costs = |h: &Harness| super::applicable_costs(h.app.world(), h.caster, &spell).unwrap();
    let before = costs(&h);
    assert!(before.iter().any(|(_, amount, _)| *amount > 0.0));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 32727,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        costs(&h).iter().all(|(_, amount, _)| *amount == 0.0),
        "source school-mask power cost pct72 makes preparation spells free"
    );
    assert_ne!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(h.caster)
            .unwrap()
            .0
            & 0x20,
        0,
        "source arena preparation unit flag"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 32727,
    );
    assert_eq!(costs(&h), before, "gate opening restores normal costs");
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitFlags>(h.caster)
            .unwrap()
            .0
            & 0x20,
        0
    );
}

#[test]
fn scaled_overkill_spells_and_periodic_ticks_use_logged_damage_units() {
    for periodic in [false, true] {
        for (health_before, expected_health, expected_overkill) in
            [(46.0, 0.0, 6), (230.0, 115.0, 0), (115.0, 0.0, 0)]
        {
            let mut h = harness(Vec::new(), &[], 8).unwrap();
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(shared::components::UnitLevel(10));
            h.app.world_mut().entity_mut(h.npc).insert((
                shared::components::UnitLevel(30),
                crate::level_scaling::tests::HOGGER_SCALING,
                crate::level_scaling::tests::hogger_stats(),
            ));
            h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = health_before;
            let spell = SpellInfo {
                id: FIREBALL,
                ..default()
            };
            let ctx = super::effects::EffectCtx {
                caster: h.caster,
                spell: &spell,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            };
            super::effects::deal_damage(h.app.world_mut(), &ctx, h.npc, 10.0, false, periodic);
            h.advance(3);
            let event = h
                .inbox
                .log
                .iter()
                .find(|event| event.kind == CombatLogKind::Damage)
                .expect("spell damage delivered to owner");
            // Logged 10 damage removes 115 native health: 46 / 115 needs 4 damage.
            assert_eq!(event.amount, 10);
            assert_eq!(
                event.overflow, expected_overkill,
                "periodic {periodic}, health before {health_before}"
            );
            assert!((0..=event.amount).contains(&event.overflow));
            assert_eq!(event.periodic, periodic);
            assert_eq!(h.health(h.npc), expected_health);
        }
    }
}

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
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(Vec::new(), &[MORTAL_STRIKE], 1)
    else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    // AP coefficient 5.2012 × attack power 100 = 520, ± 2.5% variance.
    assert!(varied(520.12).contains(&(5000.0 - h.health(h.npc))));
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
    assert!(
        varied(520.12).contains(&(damage.amount as f32)),
        "{damage:?}"
    );
    assert_eq!(damage.overflow, 0);
    let health_after_hit = 5000.0 - damage.amount as f32;

    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::OnGlobalCooldown);
    h.advance(30);
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed)[0].reason,
        CastFailReason::OnCooldown
    );
    assert_eq!(h.health(h.npc), health_after_hit);
}

#[test]
fn cast_time_spell_completes_after_its_cast_time() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    let cast = h.app.world().get::<CastState>(h.caster).expect("cast bar");
    assert_eq!(cast.duration, 1.75);
    let tick = h.app.world().resource::<Time<Fixed>>().timestep();
    assert_eq!(tick, std::time::Duration::from_millis(50));
    let observed = h.app.world().resource::<Time<Fixed>>().elapsed();
    let elapsed_ms = (f64::from(cast.elapsed) * 1000.0).round() as u64;
    let started = observed - std::time::Duration::from_millis(elapsed_ms);
    let launch_at = started + std::time::Duration::from_millis(1750);
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        10_000.0
    );
    while h.app.world().resource::<Time<Fixed>>().elapsed() + tick < launch_at {
        h.advance(1);
        assert!(h.app.world().get::<CastState>(h.caster).is_some());
        assert_eq!(
            h.app.world().get::<Mana>(h.caster).unwrap().current,
            10_000.0
        );
        assert_eq!(h.health(h.npc), 5000.0);
        assert!(
            h.inbox
                .log
                .iter()
                .all(|event| event.kind != CombatLogKind::CastSuccess)
        );
    }
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - started,
        std::time::Duration::from_millis(1700)
    );
    h.advance(1);
    assert_eq!(h.app.world().resource::<Time<Fixed>>().elapsed(), launch_at);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        9_900.0
    );
    // Launch/cost precede the native Fireball missile: minimum5yd /45yd/s
    // takes111ms, first observed at the third50ms flight tick, not at launch.
    let misc = h
        .app
        .world()
        .resource::<super::SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .misc
        .as_ref()
        .unwrap();
    assert_eq!((misc.speed, misc.launch_delay), (45.0, 0.0));
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(2);
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(1);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - launch_at,
        std::time::Duration::from_millis(150)
    );
    // SP coefficient 3.8812 × spell power 100 = 388, ± 2.5% variance.
    assert!(varied(388.12).contains(&(5000.0 - h.health(h.npc))));
    // Source1750ms =35 progress ticks, not the old float36th tick. Cost is
    // observed on the actual launch frame, before a later regen can mask it.
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
fn kick_interrupts_target_cast_and_locks_its_school_combat_log_metadata() {
    let mut h = harness(Vec::new(), &[KICK, FIREBALL], 4).expect("spell fixture");
    let before = std::time::SystemTime::now();
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
    let now = super::history_now(h.app.world());
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
    assert_eq!(
        interrupt.extra_spell_id,
        Some(FIREBALL),
        "names the interrupted spell"
    );
    assert_eq!(interrupt.school_mask, fire);
    assert_eq!(interrupt.source, Some(h.caster.to_bits()));
    assert_eq!(interrupt.target, Some(h.npc.to_bits()));
    assert_combat_log_metadata(&log, before);
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

/// WorldObject::IsValidAttackTarget (Object.cpp:2374): a UNIT_FLAG_UNINTERACTIBLE target
/// (TrinityCore's name for NOT_SELECTABLE) is not a valid attack target.
#[test]
fn spell_on_a_not_selectable_npc_fails_with_invalid_target() {
    let Some(mut h) = harness(Vec::new(), &[MORTAL_STRIKE], 1) else {
        return;
    };
    h.set_power(PowerType::Rage, 500.0);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::UnitFlags(
            shared::components::UnitFlags::NOT_SELECTABLE,
        ));
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));
    assert_eq!(
        std::mem::take(&mut h.inbox.failed),
        vec![CastFailed {
            spell_id: MORTAL_STRIKE,
            reason: CastFailReason::InvalidTarget,
            detail: None,
        }]
    );
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn threat_heal_clamps_to_max_and_logs_overheal() {
    let Some(mut h) = harness(Vec::new(), &[FLASH_HEAL], 5) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 900.0;
    let mut table = shared::components::ThreatTable::default();
    table.add_damage_threat(h.caster.to_bits(), 200.0, 1.0);
    h.app.world_mut().entity_mut(h.npc).insert(table);
    // No explicit ally target: auto self-cast.
    h.cast_and_deliver(FLASH_HEAL, None);
    h.advance(32);
    assert_eq!(h.health(h.caster), 1000.0);
    let heal = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Heal)
        .unwrap();
    // SP coefficient 6.82 × spell power 100 = 682 ± 2.5%; 100 effective.
    assert!(
        varied(682.0).contains(&(heal.amount as f32)),
        "{}",
        heal.amount
    );
    assert_eq!(heal.overflow, heal.amount - 100);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ThreatTable>(h.npc)
            .unwrap()
            .threat_for(h.caster.to_bits()),
        250.0
    );
    // The 100 effective heal adds 50 threat, never the overheal.
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
    charge_range(&mut h);
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

#[test]
fn rage_holds_through_a_lull_in_a_fight_and_decays_once_the_creature_dies() {
    let Some(mut h) = harness(Vec::new(), &[], 1) else {
        return;
    };
    let (caster, npc) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(npc)
        .insert(shared::components::ThreatTable::default());
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(CombatStatus(false));
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), caster, npc);
    h.set_power(PowerType::Rage, 500.0);
    // 8 s without a hit: the fight with a creature does not time out.
    h.advance(160);
    assert_eq!(h.power(PowerType::Rage), 500.0, "no decay in combat");

    crate::combat::combat_stop(h.app.world_mut(), npc);
    h.advance(40);
    // RegenPeace -12.5 raw/s over 2 s.
    let rage = h.power(PowerType::Rage);
    assert!((474.0..=476.0).contains(&rage), "rage {rage}");
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
    spell.effects[0].implicit_targets = [104, 16];
    let Some(mut h) = harness(vec![spell], &[], 1) else {
        return;
    };
    h.cast_and_deliver(900_001, None);
    let failed = std::mem::take(&mut h.inbox.failed);
    assert_eq!(failed[0].reason, CastFailReason::NotSupported);
    assert_eq!(
        failed[0].detail.as_deref(),
        Some("implicit target 104 not supported")
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
    // SP coefficient 3.8812 × 38 = 147.5 ± 2.5% (×2 on a 5% crit).
    let base = if damage.crit { 2.0 * 147.49 } else { 147.49 };
    assert!(varied(base).contains(&(damage.amount as f32)), "{damage:?}");
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
    assert!(varied(388.12).contains(&(5000.0 - h.health(h.npc))));
}

/// Level 80 caster knowing the real class/spec passives `passives`, 50 000
/// gtBaseMP (all mana classes at 80) and an empty 250 000 pool (regen
/// follows base mana, not the pool). Returns the
/// mana gained over 1 s in combat, then over 1 s out of combat.
fn mana_regen_per_second(class: u8, passives: &[u32]) -> Option<(f32, f32)> {
    mana_regen_at_level(class, passives, 80)
}

fn mana_regen_at_level(class: u8, passives: &[u32], level: u8) -> Option<(f32, f32)> {
    let mut h = harness(Vec::new(), passives, class)?;
    let base_mana = crate::stats::base_mana(class, level).unwrap();
    let world = h.app.world_mut();
    world
        .entity_mut(h.caster)
        .insert(shared::components::Level(level));
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
    // Native137021s4:60 points, -3/level above SpellLevel10, MaxLevel30.
    // Pinned TC CalcValue clamps the caster level first; level80 has zero bonus.
    for (level, multiplier) in [(10, 1.6), (20, 1.3), (30, 1.0), (80, 1.0)] {
        let (in_combat, out_of_combat) = mana_regen_at_level(8, &[137018, 137021], level).unwrap();
        let baseline = crate::stats::base_mana(8, level).unwrap() * 0.01;
        assert_near(in_combat, baseline * multiplier);
        assert_near(out_of_combat, baseline * multiplier);
    }
}

#[test]
fn level_10_mage_frostbolt_deals_spell_power_times_its_coefficient() {
    let Some(mut h) = harness_with(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE], 8, true) else {
        return;
    };
    // Level 10 Human Mage, no spec: the class main stat is Intellect
    // (PrimaryStatPriority 0), so spell power = Int 38.
    let stats = *h.app.world().get::<CharacterStats>(h.caster).unwrap();
    assert_eq!(stats.spell_power, 38.0);
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    h.advance(80);
    let hits: Vec<_> = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .filter(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(FROSTBOLT_DAMAGE))
        .collect();
    assert_eq!(hits.len(), 1, "{:?}", h.inbox.failed);
    // BasePoints 0 + 2.448 × 38 = 93.0 ± 2.5% (×2 on a 5% crit).
    let base = if hits[0].crit { 2.0 * 93.02 } else { 93.02 };
    assert!(
        varied(base).contains(&(hits[0].amount as f32)),
        "{:?}",
        hits[0]
    );
    assert_eq!(h.health(h.npc), 5000.0 - hits[0].amount as f32);
}

#[test]
fn a_spell_crit_doubles_the_damage() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(40);
    let hit = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .find(|e| e.kind == CombatLogKind::Damage)
        .expect("fireball hit");
    // 3.8812 × SP 100 = 388 ± 2.5%, × 2 (Unit::SpellCriticalDamageBonus).
    assert!(hit.crit);
    assert!(
        varied(2.0 * 388.12).contains(&(hit.amount as f32)),
        "{hit:?}"
    );
}

#[test]
fn physical_spell_damage_takes_the_creatures_armor() {
    let Some(mut h) = harness_with_native_enemy_dodge_reduction(Vec::new(), &[MORTAL_STRIKE], 1)
    else {
        return;
    };
    // A level 10 caster against armor 90: ArmorConstant(10) 210 stops 30%.
    let mut constants = vec![0.0; 11];
    constants[10] = 210.0;
    h.app
        .insert_resource(crate::level_scaling::ArmorConstants(constants));
    let (caster, npc) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::UnitLevel(10));
    h.app.world_mut().entity_mut(npc).insert((
        shared::components::UnitLevel(10),
        crate::level_scaling::CreatureArmor(90.0),
    ));
    h.set_power(PowerType::Rage, 500.0);
    h.cast_and_deliver(MORTAL_STRIKE, Some(npc));
    // 5.2012 × AP 100 = 520 ± 2.5% (507-533), × 0.7 truncated: 354-373.
    let damage = 5000.0 - h.health(npc);
    assert!((354.0..=373.0).contains(&damage), "{damage}");
}

#[test]
fn dormant_creature_holds_its_mana_until_its_grid_is_active_again() {
    let Some(mut h) = harness(Vec::new(), &[], 5) else {
        return;
    };
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert((
        Mana {
            current: 0.0,
            max: 100.0,
        },
        super::powers::PowerPools {
            order: vec![PowerType::Mana],
            ..Default::default()
        },
        super::powers::CreatureRegenTimer::default(),
        UnitPowers::default(),
        crate::grid_activity::GridDormant,
    ));
    // 3 s dormant: past the 2 s creature regen interval.
    h.advance(60);
    assert_eq!(h.app.world().get::<Mana>(npc).unwrap().current, 0.0);
    assert!(
        h.app
            .world()
            .get::<UnitPowers>(npc)
            .unwrap()
            .entries
            .is_empty()
    );

    h.app
        .world_mut()
        .entity_mut(npc)
        .remove::<crate::grid_activity::GridDormant>();
    h.advance(41);
    assert_eq!(h.app.world().get::<Mana>(npc).unwrap().current, 22.0);
    let entries = &h.app.world().get::<UnitPowers>(npc).unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (entries[0].power, entries[0].current, entries[0].max),
        (PowerType::Mana, 22, 100)
    );
}

/// `SpellGo`s each connection receives while `frames` frames run (lightyear clears
/// unread messages every frame).
fn receive_spell_go(h: &mut Harness, connections: &[Entity], frames: usize) -> Vec<Vec<SpellGo>> {
    let mut received = vec![Vec::new(); connections.len()];
    for _ in 0..frames {
        h.app.update();
        for (inbox, &connection) in received.iter_mut().zip(connections) {
            inbox.extend(drain::<SpellGo>(&mut h.app, connection));
        }
    }
    received
}

#[test]
fn spell_go_reaches_every_client_replicating_the_caster_with_its_hit_units() {
    let Some(mut h) =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[SLAM, BATTLE_SHOUT], 1)
    else {
        return;
    };
    h.set_power(PowerType::Rage, 1000.0);
    let observer = spawn_player(&mut h.app, 1, 2.0);
    let observer_connection = spawn_peer_connection(&mut h.app, observer, 2);
    let distant = spawn_player(&mut h.app, 1, 500.0);
    let distant_connection = spawn_peer_connection(&mut h.app, distant, 3);
    // Interest: the caster is replicated to its own client and the nearby observer.
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.connection);
    visibility.gain_visibility(observer_connection);
    h.app.world_mut().entity_mut(h.caster).insert(visibility);
    let connections = [h.connection, observer_connection, distant_connection];

    h.cast(SLAM, Some(h.npc));
    // Slam's damage lands on the target; a caster-targeted effect adds the caster.
    let slam = SpellGo {
        caster: h.caster.to_bits(),
        target: Some(h.npc.to_bits()),
        hit_targets: vec![h.npc.to_bits(), h.caster.to_bits()],
        spell_id: SLAM,
    };
    assert_eq!(
        receive_spell_go(&mut h, &connections, 5),
        vec![vec![slam.clone()], vec![slam], Vec::new()]
    );

    // Past the global cooldown: the self-buff lands on the caster.
    h.advance(40);
    h.cast(BATTLE_SHOUT, None);
    let shouts = receive_spell_go(&mut h, &connections, 5).swap_remove(1);
    assert_eq!(shouts.len(), 1, "{shouts:?}");
    assert_eq!(shouts[0].spell_id, BATTLE_SHOUT);
    assert_eq!(shouts[0].target, None);
    assert!(
        shouts[0].hit_targets.contains(&h.caster.to_bits()),
        "{shouts:?}"
    );
}

/// Each connection's messages of type `M` while `frames` frames run.
fn receive<M: lightyear::prelude::Message>(
    h: &mut Harness,
    connections: &[Entity],
    frames: usize,
) -> Vec<Vec<M>> {
    let mut received: Vec<Vec<M>> = connections.iter().map(|_| Vec::new()).collect();
    for _ in 0..frames {
        h.app.update();
        for (inbox, &connection) in received.iter_mut().zip(connections) {
            inbox.extend(drain::<M>(&mut h.app, connection));
        }
    }
    received
}

/// The caster's own client and a nearby observer replicate it; a distant client does not.
fn observed(h: &mut Harness, unit: Entity) -> [Entity; 3] {
    let observer = spawn_player(&mut h.app, 1, 2.0);
    let observer_connection = spawn_peer_connection(&mut h.app, observer, 2);
    let distant = spawn_player(&mut h.app, 1, 500.0);
    let distant_connection = spawn_peer_connection(&mut h.app, distant, 3);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.connection);
    visibility.gain_visibility(observer_connection);
    h.app.world_mut().entity_mut(unit).insert(visibility);
    [h.connection, observer_connection, distant_connection]
}

/// `Spell::update` cancels a moving caster's cast (`cancel()`, SPELL_FAILED_INTERRUPTED)
/// and `Spell::SendInterrupted` tells the caster's set (Spell.cpp:3613, 5177).
#[test]
fn moving_cancel_sends_spell_failure_to_the_casters_set() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    let caster = h.caster;
    let connections = observed(&mut h, caster);
    h.cast(FIREBALL, Some(h.npc));
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    h.app.world_mut().get_mut::<Moving>(h.caster).unwrap().0 = true;
    let failure = SpellFailure {
        caster: h.caster.to_bits(),
        spell_id: FIREBALL,
        reason: CastFailReason::Interrupted,
        failed_by: None,
    };
    assert_eq!(
        receive::<SpellFailure>(&mut h, &connections, 3),
        vec![vec![failure.clone()], vec![failure], Vec::new()]
    );
}

/// `Spell::EffectInterruptCast`: `InterruptSpell(..., SPELL_FAILED_INTERRUPTED_COMBAT, ...,
/// m_caster->GetGUID())` names the kicker as `FailedBy` (SpellEffects.cpp:3015).
#[test]
fn kick_sends_interrupted_combat_failure_naming_the_kicker() {
    let Some(mut h) = harness(Vec::new(), &[KICK, FIREBALL], 4) else {
        return;
    };
    let npc = h.npc;
    let connections = observed(&mut h, npc);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CastState::normal(FIREBALL, 0, 1.75, true));
    h.cast(KICK, Some(h.npc));
    let failure = SpellFailure {
        caster: h.npc.to_bits(),
        spell_id: FIREBALL,
        reason: CastFailReason::InterruptedCombat,
        failed_by: Some(h.caster.to_bits()),
    };
    assert_eq!(
        receive::<SpellFailure>(&mut h, &connections, 3),
        vec![vec![failure.clone()], vec![failure], Vec::new()]
    );
}

/// A cast that completes and a channel that runs out resolve without a failure.
#[test]
fn completed_casts_and_channels_send_no_spell_failure() {
    let Some(mut h) = harness(Vec::new(), &[FIREBALL], 8) else {
        return;
    };
    let npc = h.npc;
    let connections = observed(&mut h, npc);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CastState::channel(5143, 0, 0.5, 0.25, true));
    let received = receive::<SpellFailure>(&mut h, &connections, 60);
    assert!(
        h.app.world().get::<CastState>(h.npc).is_none(),
        "channel ran out"
    );
    assert_eq!(received, vec![Vec::new(), Vec::new(), Vec::new()]);
}
