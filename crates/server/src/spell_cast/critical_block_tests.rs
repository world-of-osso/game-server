//! Native Protection mastery76857 chance253; actual blocked player swings.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::melee::PlayerDefense;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::{Rotation, UnitLevel};
use shared::protocol::{CombatLogEvent, CombatLogKind};

const CRITICAL_BLOCK: u32 = 76857;

fn shield_duel(mastery_points: f32) -> (Harness, Entity) {
    shield_duel_for_class(mastery_points, 1)
}

fn shield_duel_for_class(mastery_points: f32, caster_class: u8) -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[CRITICAL_BLOCK, 203524, 1464, 17, 853, 7384, 51723, 19434],
        caster_class,
    )
    .unwrap();
    let defender = spawn_player(&mut h.app, 1, 3.0);
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::duel::DuelOpponent(defender),
        // Native player entry inserts matching owner/combat levels.
        UnitLevel(10),
        crate::class_progression::KnownSpells::fixture(Vec::new()),
    ));
    h.app.world_mut().entity_mut(defender).insert((
        crate::duel::DuelOpponent(h.caster),
        UnitLevel(10),
        Rotation {
            x: 0.0,
            y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    load_native_armor_constants(&mut h);
    h.advance(2);
    equip_shield_and_weapon(&mut h, defender, mastery_points);
    (h, defender)
}

fn load_native_armor_constants(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let constants = crate::world_data::load_expected_stats(&conn).armor_constants();
    assert!(
        constants.at(10).unwrap() > 1.0,
        "native level10 armor constant required"
    );
    h.app.insert_resource(constants);
}

fn equip_shield_and_weapon(h: &mut Harness, defender: Entity, mastery_points: f32) {
    let armor_constant = h
        .app
        .world()
        .resource::<crate::level_scaling::ArmorConstants>()
        .at(10)
        .unwrap();
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<CharacterStats>(defender)
        .unwrap();
    stats.mastery_points = mastery_points;
    stats.defense = PlayerDefense {
        block: 100.0,
        shield_block: armor_constant,
        ..Default::default()
    };
    drop(stats);
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap();
    stats.attack_power = 0.0;
    stats.crit_pct = 0.0;
    stats.main_hand.min = 333.0;
    stats.main_hand.max = 333.0;
}

fn apply_mastery(h: &mut Harness, unit: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CRITICAL_BLOCK,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn blocked_swing(h: &mut Harness, defender: Entity) -> (i32, i32, bool) {
    h.inbox = Inbox::default();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(defender));
    for _ in 0..120 {
        h.advance(1);
        let hit = h.inbox.log.iter().find(|event| {
            event.source == Some(h.caster.to_bits())
                && event.target == Some(defender.to_bits())
                && event.spell_id.is_none()
                && event.kind == CombatLogKind::Damage
        });
        if let Some(hit) = hit {
            let result = (hit.amount, hit.blocked, hit.crit);
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .remove::<crate::combat::AutoAttack>();
            return result;
        }
    }
    panic!("no real blocked swing in six seconds: {:?}", h.inbox.log);
}

#[test]
fn class_completeness_native_critical_block_doubles_already_truncated_autoattack_block() {
    let (mut h, defender) = shield_duel(100.0);
    apply_mastery(&mut h, defender);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(defender)
            .unwrap()
            .modifier(253),
        150.0
    );
    // 333 ×50%=166.5 truncates to166 BEFORE doubling, TC Unit.cpp1469–1473.
    assert_eq!(blocked_swing(&mut h, defender), (1, 332, false));
}

#[test]
fn class_completeness_native_zero_critical_block_keeps_ordinary_block() {
    let (mut h, defender) = shield_duel(0.0);
    apply_mastery(&mut h, defender);
    assert_eq!(blocked_swing(&mut h, defender), (167, 166, false));
}

#[test]
fn class_completeness_native_attacker_critical_block_aura_does_not_protect_defender() {
    let (mut h, defender) = shield_duel(0.0);
    let caster = h.caster;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(caster)
        .unwrap()
        .mastery_points = 100.0;
    apply_mastery(&mut h, caster);
    assert_eq!(blocked_swing(&mut h, defender), (167, 166, false));
}

#[test]
fn class_completeness_native_critical_block_removal_restores_ordinary_block() {
    let (mut h, defender) = shield_duel(100.0);
    apply_mastery(&mut h, defender);
    assert_eq!(blocked_swing(&mut h, defender), (1, 332, false));
    crate::auras::remove_auras(
        h.app.world_mut(),
        defender,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == CRITICAL_BLOCK,
    );
    assert_eq!(blocked_swing(&mut h, defender), (167, 166, false));
}

#[test]
fn class_completeness_native_finite_critical_block_field_expires_on_its_source_clock() {
    let (mut h, defender) = shield_duel(0.0);
    // Neltharion's Fury203524 effect2: native100%, Duration3000ms.
    // This isolates that aura field, not the parent channel's other effects.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: defender,
            target: defender,
            spell_id: 203524,
            effect_mask: 1 << 2,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(blocked_swing(&mut h, defender), (1, 332, false));
    h.advance(65);
    assert!(!crate::auras::has_aura(h.app.world(), defender, 203524));
    assert_eq!(blocked_swing(&mut h, defender), (167, 166, false));
}

#[test]
fn class_completeness_native_slam_rolls_block_and_critical_block_before_absorbs() {
    for critical in [false, true] {
        let (mut h, defender) = shield_duel(if critical { 100.0 } else { 0.0 });
        let caster = h.caster;
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert(crate::class_progression::KnownSpells::fixture(vec![1464]));
        h.app
            .world_mut()
            .get_mut::<CharacterStats>(caster)
            .unwrap()
            .attack_power = 100.0;
        h.set_power(shared::components::PowerType::Rage, 1000.0);
        apply_mastery(&mut h, defender);
        crate::auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: defender,
                target: defender,
                spell_id: 17,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
        h.inbox = Inbox::default();
        let health = h.health(defender);
        h.cast_and_deliver(1464, Some(defender));
        h.advance(3);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let hits: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(1464)
                    && event.target == Some(defender.to_bits())
                    && event.kind == CombatLogKind::Damage
            })
            .collect();
        assert_eq!(hits.len(), 1);
        let hit = hits[0];
        let total = hit.amount + hit.blocked + hit.absorbed;
        // Active1464 effect462:3.03484988AP,5% variance. Fixture100AP.
        assert!(varied(303.0).contains(&(total as f32)), "{hit:?}");
        assert!(!hit.crit, "critical block is not a critical strike");
        if critical {
            // Spell blocks double50% BEFORE truncation: full block, unlike autoswings.
            assert_eq!(hit.amount, 0, "{hit:?}");
            assert_eq!(
                hit.absorbed, 0,
                "full block precedes and preserves the shield"
            );
            assert_eq!(h.health(defender), health);
        } else {
            assert_eq!(hit.blocked, (total as f32 * 0.5).trunc() as i32, "{hit:?}");
            assert!(
                hit.absorbed > 0,
                "ordinary block must reduce damage before absorb"
            );
            assert_eq!(health - h.health(defender), hit.amount as f32);
        }
    }
}

fn start_damage_spell(h: &mut Harness, defender: Entity, spell: u32) {
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![spell]));
    {
        let mut stats = h.app.world_mut().get_mut::<CharacterStats>(caster).unwrap();
        stats.attack_power = 100.0;
        stats.ranged_attack_power = 100.0;
    }
    let power = h
        .app
        .world()
        .get::<super::powers::PowerPools>(caster)
        .unwrap()
        .order[0];
    let maximum = super::powers::power_value(h.app.world(), caster, power)
        .unwrap()
        .1;
    h.set_power(power, maximum);
    h.inbox = Inbox::default();
    h.cast_and_deliver(spell, Some(defender));
}

fn cast_damage_spell(
    h: &mut Harness,
    defender: Entity,
    spell: u32,
    frames: usize,
) -> Vec<shared::protocol::CombatLogEvent> {
    start_damage_spell(h, defender, spell);
    h.advance(frames);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(spell)
                && event.target == Some(defender.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .cloned()
        .collect()
}

#[test]
fn class_completeness_native_player_spell_block_requires_frontal_direction() {
    let (mut h, defender) = shield_duel(100.0);
    apply_mastery(&mut h, defender);
    h.app.world_mut().get_mut::<Rotation>(defender).unwrap().y = std::f32::consts::FRAC_PI_2;
    let hits = cast_damage_spell(&mut h, defender, 1464, 3);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].blocked, 0);
    assert!(varied(303.0).contains(&(hits[0].amount as f32)));
}

#[test]
fn class_completeness_native_stunned_player_cannot_block_weapon_spell() {
    let (mut h, defender) = shield_duel(100.0);
    apply_mastery(&mut h, defender);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: defender,
            spell_id: 853,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(defender)
            .unwrap()
            .is_stunned()
    );
    let hits = cast_damage_spell(&mut h, defender, 1464, 3);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].blocked, 0);
    assert!(hits[0].amount > 0);
}

#[test]
fn class_completeness_native_weapon_spell_no_active_defense_and_no_block_flags_are_honored() {
    // Native Overpower7384 Attr0NoActiveDefense; FanOfKnives51723 Attr8NoAttackBlock.
    // ShatteringThrow64382 is a dispel-only parent with a separate unbound damage child.
    for (spell, frames, class) in [(7384, 3, 1), (51723, 20, 4)] {
        let (mut h, defender) = shield_duel_for_class(100.0, class);
        apply_mastery(&mut h, defender);
        let hits = cast_damage_spell(&mut h, defender, spell, frames);
        assert_eq!(hits.len(), 1, "{spell}: {hits:?}");
        assert_eq!(hits[0].blocked, 0, "{spell}: {:?}", hits[0]);
        assert!(hits[0].amount > 0);
    }
}

fn wait_for_aimed_launch(h: &mut Harness) {
    for _ in 0..65 {
        if h.inbox.go.iter().any(|event| event.spell_id == 19434) {
            return;
        }
        h.advance(1);
    }
    panic!("native Aimed Shot never launched: {:?}", h.inbox.failed);
}

fn aimed_damage(h: &Harness, defender: Entity) -> Vec<&shared::protocol::CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(19434)
                && event.target == Some(defender.to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .collect()
}

#[test]
fn class_completeness_native_weapon_spell_block_survives_flight_and_facing_change() {
    let (mut h, defender) = shield_duel_for_class(0.0, 3);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(defender)
        .unwrap()
        .x = 40.0;
    start_damage_spell(&mut h, defender, 19434);
    wait_for_aimed_launch(&mut h);
    assert!(
        aimed_damage(&h, defender).is_empty(),
        "must observe launch before impact"
    );
    h.app.world_mut().get_mut::<Rotation>(defender).unwrap().y = std::f32::consts::FRAC_PI_2;
    h.advance(12);
    let hits = aimed_damage(&h, defender);
    assert_eq!(hits.len(), 1);
    let total = hits[0].amount + hits[0].blocked;
    assert!(total > 0);
    assert_eq!(hits[0].blocked, (total as f32 * 0.5).trunc() as i32);
}

#[test]
fn class_completeness_native_critical_block_chance_is_read_at_missile_impact() {
    let (mut h, defender) = shield_duel_for_class(0.0, 3);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(defender)
        .unwrap()
        .x = 40.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: defender,
            target: defender,
            spell_id: 203524,
            effect_mask: 1 << 2,
            source: AuraSource::Cast,
        },
    );
    let applied_at = h.app.world().resource::<Time<Fixed>>().elapsed();
    println!(
        "critical-block aura applied at{applied_at:?}: {:?}",
        h.app.world().get::<ActiveAuras>(defender).unwrap()
    );
    // Native2500ms cast +400ms flight; loopback3frames makes the3s buff
    // expire after launch but before impact, without changing spell metadata.
    start_damage_spell(&mut h, defender, 19434);
    println!(
        "critical-block shot received at{:?}: {:?}",
        h.app.world().resource::<Time<Fixed>>().elapsed(),
        h.app.world().get::<shared::casting::CastState>(h.caster)
    );
    wait_for_aimed_launch(&mut h);
    println!(
        "critical-block shot launched at{:?}",
        h.app.world().resource::<Time<Fixed>>().elapsed()
    );
    assert!(crate::auras::has_aura(h.app.world(), defender, 203524));
    assert!(aimed_damage(&h, defender).is_empty());
    for _ in 0..12 {
        h.advance(1);
        println!(
            "critical-block flight at{:?}, aura{}, hits{:?}",
            h.app.world().resource::<Time<Fixed>>().elapsed(),
            crate::auras::has_aura(h.app.world(), defender, 203524),
            aimed_damage(&h, defender)
        );
    }
    assert!(!crate::auras::has_aura(h.app.world(), defender, 203524));
    let hits = aimed_damage(&h, defender);
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].amount > 0,
        "expired chance must not produce a full critical block"
    );
    let total = hits[0].amount + hits[0].blocked;
    assert_eq!(hits[0].blocked, (total as f32 * 0.5).trunc() as i32);
}

fn creature_block_duel(entry: u32, class: u8) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[1464, 10021, 203524, 853, 3248, 3650, 19434, 87322, 45769],
        class,
    )
    .unwrap();
    let data = load_native_creature_world(&[entry]);
    let template = &data.templates[&entry];
    let name = template.name.clone();
    let faction = template.faction;
    let level = template.min_level;
    h.app.insert_resource(data);
    // Isolate block without a high-level NPC's legitimate level-based dodge.
    let caster_level = level.max(10);
    h.app.world_mut().entity_mut(h.caster).insert((
        shared::components::Level(caster_level),
        UnitLevel(caster_level),
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        shared::components::Npc {
            template_id: entry,
            name,
        },
        shared::components::UnitFactionTemplate(u32::from(faction)),
        UnitLevel(level),
        Rotation {
            x: 0.0,
            y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    // Permanent native Sunwell Radiance effect1 reduces enemy dodge by20 points.
    // Keep baseline dodge out of this block-only fixture, without changing RNG.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 45769,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .modifier(251),
        -20.0
    );
    // Native10021 effect0 gives100 block percentage points to a creature.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id: 10021,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h
}

#[test]
fn class_completeness_native_creature_spell_block_uses_creature_percent_not_player_shield() {
    for critical in [false, true] {
        let mut h = creature_block_duel(6, 1);
        let target = h.npc;
        if critical {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: target,
                    target,
                    spell_id: 203524,
                    effect_mask: 1 << 2,
                    source: AuraSource::Cast,
                },
            );
        }
        let hits = cast_damage_spell(&mut h, target, 1464, 3);
        assert_eq!(hits.len(), 1);
        let total = hits[0].amount + hits[0].blocked;
        assert!(varied(303.0).contains(&(total as f32)));
        // Creature::GetBlockPercent is30%, critical doubles it to60%.
        let percent = if critical { 0.6 } else { 0.3 };
        assert_eq!(
            hits[0].blocked,
            (total as f32 * percent).trunc() as i32,
            "{:?}",
            hits[0]
        );
    }
}

#[test]
fn class_completeness_native_creature_no_block_flag_overrides_block_aura() {
    // Native36980 Ice Tomb has NO_BLOCK0x10; 1511 has only NO_XP0x40.
    let mut h = creature_block_duel(36980, 1);
    let target = h.npc;
    let hits = cast_damage_spell(&mut h, target, 1464, 3);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].blocked, 0);
    assert!(varied(303.0).contains(&(hits[0].amount as f32)));
}

#[test]
fn class_completeness_native_creature_block_requires_facing_and_active_defense() {
    for stunned in [false, true] {
        let mut h = creature_block_duel(6, 1);
        let target = h.npc;
        if stunned {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: h.caster,
                    target,
                    spell_id: 853,
                    effect_mask: 1,
                    source: AuraSource::Cast,
                },
            );
            assert!(
                h.app
                    .world()
                    .get::<ActiveAuras>(target)
                    .unwrap()
                    .is_stunned()
            );
        } else {
            h.app.world_mut().get_mut::<Rotation>(target).unwrap().y = std::f32::consts::FRAC_PI_2;
        }
        let hits = cast_damage_spell(&mut h, target, 1464, 3);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].blocked, 0);
        assert!(varied(303.0).contains(&(hits[0].amount as f32)));
    }
}

#[test]
fn class_completeness_native_creature_no_xp_flag_does_not_disable_block() {
    let mut h = creature_block_duel(1511, 1);
    let target = h.npc;
    let hits = cast_damage_spell(&mut h, target, 1464, 3);
    assert_eq!(hits.len(), 1);
    let total = hits[0].amount + hits[0].blocked;
    assert!(varied(303.0).contains(&(total as f32)));
    assert_eq!(hits[0].blocked, (total as f32 * 0.3).trunc() as i32);
}

#[test]
fn class_completeness_native_weapon_spell_miss_and_block_share_one_hit_table() {
    let mut h = creature_block_duel(6, 1);
    let target = h.npc;
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 10021,
    );
    // Native3650 reduces caster hit by50;3248 adds55 to NPC block's3.
    // Their cumulative table fills100%: no random draw can be an unblocked hit.
    for attempt in 0..60 {
        h.advance(40);
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap()
            .current = 5000.0;
        for (recipient, spell_id) in [(h.caster, 3650), (target, 3248)] {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: recipient,
                    target: recipient,
                    spell_id,
                    effect_mask: 1,
                    source: AuraSource::Cast,
                },
            );
        }
        let hits = cast_damage_spell(&mut h, target, 1464, 3);
        if let Some(hit) = hits.first() {
            assert_eq!(hits.len(), 1);
            assert!(
                hit.blocked > 0,
                "native cumulative table admitted unblocked hit at{attempt}: {hit:?}"
            );
        } else {
            assert!(
                h.inbox.log.iter().any(|event| {
                    event.spell_id == Some(1464)
                        && event.target == Some(target.to_bits())
                        && event.kind == CombatLogKind::Miss(shared::protocol::MissKind::Miss)
                }),
                "no actual native miss/block result at{attempt}: {:?}",
                h.inbox.log
            );
        }
    }
}

#[test]
fn class_completeness_native_ranged_deflection_and_block_share_one_hit_table() {
    let mut h = creature_block_duel(6, 3);
    let target = h.npc;
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 10021,
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap()
        .x = 40.0;
    // Native87322 gives50 deflection;3248 gives55 plusNPC3 block.
    // Actual Hunter Aimed Shot must be deflected or partially blocked for every draw.
    for attempt in 0..60 {
        h.advance(300);
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap()
            .current = 5000.0;
        for spell_id in [3248, 87322] {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: target,
                    target,
                    spell_id,
                    effect_mask: 1,
                    source: AuraSource::Cast,
                },
            );
        }
        let hits = cast_damage_spell(&mut h, target, 19434, 65);
        if let Some(hit) = hits.first() {
            assert_eq!(hits.len(), 1);
            assert!(
                hit.blocked > 0,
                "native ranged table admitted unblocked hit at{attempt}: {hit:?}"
            );
        } else {
            assert!(
                h.inbox.log.iter().any(|event| {
                    event.spell_id == Some(19434)
                        && event.target == Some(target.to_bits())
                        && event.kind == CombatLogKind::Miss(shared::protocol::MissKind::Deflect)
                }),
                "no actual native deflect/block result at{attempt}: {:?}",
                h.inbox.log
            );
        }
    }
}

fn npc_evasion_duel() -> (Harness, Entity) {
    let (mut h, defender) = evasion_duel(1);
    // NPC/player receipts are delivered to the actual player target's owner.
    h.connection = spawn_peer_connection(&mut h.app, defender, 2);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 0.0;
    (h, defender)
}

fn cast_npc_strike(h: &mut Harness, defender: Entity) -> Vec<CombatLogEvent> {
    h.inbox.log.clear();
    super::creature_cast(h.app.world_mut(), h.npc, 38598, Some(defender), true)
        .expect("native creature Strike cast");
    h.advance(2);
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(38598)
                && event.kind == CombatLogKind::Damage
                && event.target == Some(defender.to_bits())
        })
        .cloned()
        .collect()
}

#[test]
fn class_completeness_native_npc_expertise_overcomes_evasion_until_removed() {
    let (mut h, defender) = npc_evasion_duel();
    super::creature_cast(h.app.world_mut(), h.npc, 62627, Some(h.npc), true)
        .expect("native NPC Hit Buffs self-cast");
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 62627));
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(h.npc)
            .unwrap()
            .modifier(240),
        10000.0,
        "native NPC expertise amount must reach its consumer"
    );
    let before = h.health(defender);
    let hits = cast_npc_strike(&mut h, defender);
    assert_eq!(
        hits.len(),
        1,
        "native expertise must overcome Evasion; receipts: {:?}",
        h.inbox.log
    );
    assert!(hits[0].amount > 0);
    assert_eq!(before - h.health(defender), hits[0].amount as f32);
    assert!(crate::auras::has_aura(h.app.world(), defender, 5277));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 62627,
    );
    h.advance(2);
    let before = h.health(defender);
    assert!(cast_npc_strike(&mut h, defender).is_empty());
    assert_eq!(h.health(defender), before);
}

#[test]
fn class_completeness_native_npc_expertise_on_foreign_actor_does_not_reduce_dodge() {
    let (mut h, defender) = npc_evasion_duel();
    let foreign = spawn_npc(&mut h.app, 1.0);
    super::creature_cast(h.app.world_mut(), foreign, 62627, Some(foreign), true)
        .expect("foreign native NPC Hit Buffs self-cast");
    h.advance(2);
    let before = h.health(defender);
    assert!(cast_npc_strike(&mut h, defender).is_empty());
    assert_eq!(h.health(defender), before);
    assert!(crate::auras::has_aura(h.app.world(), foreign, 62627));
}

fn evasion_duel(class: u8) -> (Harness, Entity) {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[5277, 1464, 7384, 19434, 51723, 853, 62627, 38598],
        class,
        false,
        |app| {
            app.add_plugins(crate::stats::StatsPlugin);
        },
    )
    .unwrap();
    let defender = spawn_player(&mut h.app, 4, 3.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((crate::duel::DuelOpponent(defender), UnitLevel(10)));
    h.app
        .world_mut()
        .entity_mut(defender)
        .remove::<CharacterStats>()
        .insert((
            crate::duel::DuelOpponent(h.caster),
            UnitLevel(10),
            Rotation {
                x: 0.0,
                y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            crate::class_progression::KnownSpells::fixture(vec![5277]),
            // StatsPlugin derives the receiver's stats from native class/level inputs.
            shared::components::UnitStats::default(),
            shared::components::CombatRatings::default(),
            shared::components::DerivedStats::default(),
        ));
    // Actual Rogue self-cast, not a synthetic dodge amount or fake NPC profile.
    let spell = world_db().unwrap().store.get(5277).unwrap();
    super::execute_cast(h.app.world_mut(), defender, spell, Some(defender)).unwrap();
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), defender, 5277));
    assert!(
        h.app
            .world()
            .get::<CharacterStats>(defender)
            .unwrap()
            .defense
            .dodge
            >= 100.0,
        "native Evasion must supply guaranteed dodge before exercising its consumer"
    );
    (h, defender)
}

#[test]
fn class_completeness_native_evasion_dodges_frontal_weapon_spell() {
    let (mut h, defender) = evasion_duel(1);
    let before = h.health(defender);
    let hits = cast_damage_spell(&mut h, defender, 1464, 3);
    assert!(hits.is_empty(), "native Evasion admitted damage: {hits:?}");
    assert_eq!(h.health(defender), before);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(1464)
                && event.target == Some(defender.to_bits())
                && event.kind == CombatLogKind::Miss(shared::protocol::MissKind::Dodge)
        }),
        "{:?}",
        h.inbox.log
    );
}

#[test]
fn class_completeness_native_evasion_does_not_dodge_attacks_from_behind() {
    let (mut h, defender) = evasion_duel(1);
    h.app.world_mut().get_mut::<Rotation>(defender).unwrap().y = std::f32::consts::FRAC_PI_2;
    let hits = cast_damage_spell(&mut h, defender, 1464, 3);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].amount > 0);
    assert!(crate::auras::has_aura(h.app.world(), defender, 5277));
}

#[test]
fn class_completeness_native_evasion_cannot_dodge_while_stunned() {
    let (mut h, defender) = evasion_duel(1);
    let paladin = spawn_player(&mut h.app, 2, 1.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: paladin,
            target: defender,
            spell_id: 853,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(defender)
            .unwrap()
            .is_stunned()
    );
    let hits = cast_damage_spell(&mut h, defender, 1464, 3);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].amount > 0);
}

#[test]
fn class_completeness_native_evasion_honors_no_active_defense_and_no_attack_dodge() {
    for (class, spell) in [(1, 7384), (4, 51723)] {
        let (mut h, defender) = evasion_duel(class);
        let hits = cast_damage_spell(&mut h, defender, spell, 3);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].amount > 0);
        assert!(crate::auras::has_aura(h.app.world(), defender, 5277));
    }
}

#[test]
fn class_completeness_native_evasion_does_not_dodge_ranged_aimed_shot() {
    let (mut h, defender) = evasion_duel(3);
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(defender)
        .unwrap()
        .x = 40.0;
    let hits = cast_damage_spell(&mut h, defender, 19434, 65);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].amount > 0);
    assert!(crate::auras::has_aura(h.app.world(), defender, 5277));
}

#[test]
fn class_completeness_native_evasion_removal_and_expiry_restore_weapon_hits() {
    for expires in [false, true] {
        let (mut h, defender) = evasion_duel(1);
        if expires {
            h.advance(205); // Native5277 duration10000ms.
        } else {
            crate::auras::remove_auras(
                h.app.world_mut(),
                defender,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 5277,
            );
            h.advance(2);
        }
        assert!(!crate::auras::has_aura(h.app.world(), defender, 5277));
        assert_eq!(
            h.app
                .world()
                .get::<CharacterStats>(defender)
                .unwrap()
                .defense
                .dodge,
            0.0
        );
        let hits = cast_damage_spell(&mut h, defender, 1464, 3);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].amount > 0);
    }
}
