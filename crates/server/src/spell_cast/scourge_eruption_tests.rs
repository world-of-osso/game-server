//! Native70448 Scourge Strike55090, SimC76f8118 sc_death_knight12502–12661.
//! Periodic-only aura setup is not Outbreak/spread or a legal talent-loadout certificate.
use super::test_harness::*;
use crate::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::Level;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const STRIKE: u32 = 55090;
const VIRULENT: u32 = 191587;
const DREAD: u32 = 1240996;
const VIRULENT_ERUPT: u32 = 1241167;
const DREAD_ERUPT: u32 = 1241171;
const MARK: u32 = 1126;
const HAUNT: u32 = 48181;
const SHADOW_IMMUNITY: u32 = 7743;
const SHIELD: u32 = 17;
const VIRULENT_AP: f32 = 0.253_567_994;
const DREAD_AP: f32 = 0.548_887_014;

fn fixture(level: u8) -> Harness {
    let ids = [
        STRIKE,
        VIRULENT,
        DREAD,
        VIRULENT_ERUPT,
        DREAD_ERUPT,
        MARK,
        HAUNT,
        SHADOW_IMMUNITY,
        SHIELD,
        SUNWELL_RADIANCE,
    ];
    let mut h = harness_with(Vec::new(), &ids, 6, true).unwrap();
    h.app.insert_resource(load_native_creature_world(&[6]));
    h.app.world_mut().entity_mut(h.caster).insert(Level(level));
    apply_native_enemy_dodge_reduction(&mut h);
    h.advance(4);
    h
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell: u32, mask: u32) {
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

fn plague(h: &Harness, caster: Entity, spell: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|a| a.caster == caster && a.spell_id == spell)
        .unwrap()
        .clone()
}

fn damage<'a>(events: &'a [CombatLogEvent], caster: Entity, spell: u32) -> Vec<&'a CombatLogEvent> {
    events
        .iter()
        .filter(|e| {
            e.source == Some(caster.to_bits())
                && e.spell_id == Some(spell)
                && e.kind == CombatLogKind::Damage
        })
        .collect()
}

fn assert_eruption(h: &Harness, spell: u32, coefficient: f32, multiplier: f32) {
    let events = damage(&h.inbox.log, h.caster, spell);
    assert_eq!(events.len(), 1, "native eruption child {spell}");
    let event = events[0];
    let ap = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    let critical = if event.crit { 2.0 } else { 1.0 };
    assert_eq!(
        event.amount as f32,
        (ap * coefficient * 0.35 * multiplier * critical).round()
    );
    assert!(!event.periodic);
    assert_eq!(event.target, Some(h.npc.to_bits()));
}

fn strike(h: &mut Harness) {
    h.cast_and_deliver(STRIKE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        damage(&h.inbox.log, h.caster, STRIKE).len(),
        1,
        "actual root lands"
    );
}

#[test]
fn unholy_scourge_native_owned_plagues_erupt_one_tick_through_their_actual_children() {
    for spells in [&[VIRULENT][..], &[DREAD][..], &[VIRULENT, DREAD][..]] {
        let mut h = fixture(80);
        let caster = h.caster;
        for &spell in spells {
            let target = h.npc;
            apply(&mut h, caster, target, spell, 1);
        }
        let health = h.health(h.npc);
        strike(&mut h);
        if spells.contains(&VIRULENT) {
            assert_eruption(&h, VIRULENT_ERUPT, VIRULENT_AP, 1.0);
        } else {
            assert!(damage(&h.inbox.log, caster, VIRULENT_ERUPT).is_empty());
        }
        if spells.contains(&DREAD) {
            assert_eruption(&h, DREAD_ERUPT, DREAD_AP, 1.0);
        } else {
            assert!(damage(&h.inbox.log, caster, DREAD_ERUPT).is_empty());
        }
        let total: i32 = h
            .inbox
            .log
            .iter()
            .filter(|e| e.kind == CombatLogKind::Damage)
            .map(|e| e.amount)
            .sum();
        assert_eq!(health - h.health(h.npc), total as f32);
    }
}

#[test]
fn unholy_scourge_native_missing_and_foreign_plagues_do_not_erupt_newly_applied_virulent() {
    for foreign in [false, true] {
        let mut h = fixture(80);
        let other = spawn_player(&mut h.app, 6, 0.0);
        if foreign {
            let target = h.npc;
            apply(&mut h, other, target, VIRULENT, 1);
            apply(&mut h, other, target, DREAD, 1);
        }
        let before = foreign.then(|| plague(&h, other, VIRULENT));
        strike(&mut h);
        assert!(damage(&h.inbox.log, h.caster, VIRULENT_ERUPT).is_empty());
        assert!(damage(&h.inbox.log, h.caster, DREAD_ERUPT).is_empty());
        assert_eq!(plague(&h, h.caster, VIRULENT).caster, h.caster);
        if let Some(before) = before {
            assert_eq!(plague(&h, other, VIRULENT).expires_at, before.expires_at);
        }
    }
}

#[test]
fn unholy_scourge_native_dread_erupt_precedes_missing_virulent_without_first_hit_eruption() {
    let mut h = fixture(80);
    let (caster, target) = (h.caster, h.npc);
    apply(&mut h, caster, target, DREAD, 1);
    strike(&mut h);
    assert_eruption(&h, DREAD_ERUPT, DREAD_AP, 1.0);
    assert!(damage(&h.inbox.log, caster, VIRULENT_ERUPT).is_empty());
    assert_eq!(plague(&h, caster, VIRULENT).caster, caster);
}

#[test]
fn unholy_scourge_native_eruptions_preserve_expiry_phase_and_next_real_tick() {
    let mut h = fixture(80);
    let (caster, target) = (h.caster, h.npc);
    apply(&mut h, caster, target, VIRULENT, 1);
    let before = plague(&h, caster, VIRULENT);
    h.advance(20);
    strike(&mut h);
    assert_eruption(&h, VIRULENT_ERUPT, VIRULENT_AP, 1.0);
    let after = plague(&h, caster, VIRULENT);
    assert_eq!(after.expires_at, before.expires_at);
    h.advance(35);
    assert!(damage(&h.inbox.log, caster, VIRULENT).is_empty());
    h.advance(4);
    let ticks = damage(&h.inbox.log, caster, VIRULENT);
    assert_eq!(ticks.len(), 1);
    assert!(ticks[0].periodic);
    let ap = h
        .app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .attack_power;
    assert_eq!(ticks[0].amount as f32, (ap * VIRULENT_AP).round());
}

#[test]
fn unholy_scourge_native_source_versatility_once_and_nonbug_target_multiplier_twice() {
    let mut h = fixture(80);
    let (caster, target) = (h.caster, h.npc);
    apply(&mut h, caster, caster, MARK, 1);
    apply(&mut h, caster, target, HAUNT, 2);
    h.advance(4);
    assert_eq!(
        h.app
            .world()
            .get::<CharacterStats>(caster)
            .unwrap()
            .versatility_pct,
        3.0
    );
    apply(&mut h, caster, target, VIRULENT, 1);
    apply(&mut h, caster, target, DREAD, 1);
    strike(&mut h);
    // SimC non-bug source tick retains target_ta_multiplier; native children
    // ignore caster percent modifiers, not their own victim-taken stage.
    assert_eruption(&h, VIRULENT_ERUPT, VIRULENT_AP, 1.03 * 1.16 * 1.16);
    assert_eruption(&h, DREAD_ERUPT, DREAD_AP, 1.03 * 1.16 * 1.16);
}

#[test]
fn unholy_scourge_native_removed_plague_cannot_erupt_after_fresh_application() {
    let mut h = fixture(80);
    let (caster, target) = (h.caster, h.npc);
    apply(&mut h, caster, target, VIRULENT, 1);
    auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |a| {
        a.spell_id == VIRULENT
    });
    strike(&mut h);
    assert!(damage(&h.inbox.log, caster, VIRULENT_ERUPT).is_empty());
    assert_eq!(plague(&h, caster, VIRULENT).caster, caster);
}

#[test]
fn unholy_scourge_native_shadow_immunity_blocks_root_and_children() {
    let mut h = fixture(80);
    let (caster, target) = (h.caster, h.npc);
    apply(&mut h, caster, target, VIRULENT, 1);
    apply(&mut h, caster, target, SHADOW_IMMUNITY, u32::MAX);
    let health = h.health(target);
    h.cast_and_deliver(STRIKE, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(target), health);
    assert!(damage(&h.inbox.log, caster, VIRULENT_ERUPT).is_empty());
    assert!(damage(&h.inbox.log, caster, STRIKE).is_empty());
}

#[test]
fn unholy_scourge_native_shield_absorbs_eruption_without_consuming_periodic_damage() {
    let mut h = fixture(10);
    let (caster, target) = (h.caster, h.npc);
    let priest = spawn_player(&mut h.app, 5, 0.0);
    apply(&mut h, caster, target, VIRULENT, 1);
    apply(&mut h, priest, target, SHIELD, u32::MAX);
    let health = h.health(target);
    strike(&mut h);
    let events = damage(&h.inbox.log, caster, VIRULENT_ERUPT);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 0);
    assert!(events[0].absorbed > 0);
    assert_eq!(h.health(target), health);
    assert_eq!(plague(&h, caster, VIRULENT).stacks, 1);
}
