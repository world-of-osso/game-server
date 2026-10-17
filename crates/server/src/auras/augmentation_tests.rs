//! Active70448 rows; SimC76f8118 sc_evoker.cpp7611/8538/8968.
//! Explicit friendly Prescience and the Sands aura boundary only, not auto-target
//! prioritization, empowered-spell dispatch, E202 or the other dummy fields.
use bevy::prelude::*;
use shared::components::{CombatRatings, DerivedStats, Health, Level, UnitStats};
use shared::protocol::CombatLogKind;

use super::*;
use crate::class_progression::ActiveSpec;
use crate::spell_cast::test_harness::{Harness, harness_with, spawn_player, varied, world_db};
use crate::stats::CharacterStats;

const PRESCIENCE: u32 = 409_311;
const PRESCIENCE_BUFF: u32 = 410_089;
const SHIFTING_SANDS: u32 = 413_984;
const MASTERY: u32 = 406_380;
const ELEMENTAL_BLAST_MASTERY: u32 = 173_184;
const DAMAGE: u32 = 314_633;
const HEAL: u32 = 2061;

fn fixture() -> (Harness, Entity) {
    let mut h = harness_with(
        Vec::new(),
        &[
            PRESCIENCE,
            PRESCIENCE_BUFF,
            SHIFTING_SANDS,
            MASTERY,
            ELEMENTAL_BLAST_MASTERY,
            DAMAGE,
            HEAL,
        ],
        13,
        true,
    )
    .unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(1473)));
    let ally = spawn_computed_player(&mut h, 8, 62, 3.0);
    h.advance(4);
    assert!(stats(&h, h.caster).mastery_pct > 0.0);
    (h, ally)
}

fn spawn_computed_player(h: &mut Harness, class: u8, spec: u32, x: f32) -> Entity {
    let unit = spawn_player(&mut h.app, class, x);
    h.app
        .world_mut()
        .entity_mut(unit)
        .remove::<CharacterStats>()
        .insert((
            Level(80),
            ActiveSpec(spec),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
        ));
    unit
}

fn stats(h: &Harness, unit: Entity) -> CharacterStats {
    *h.app.world().get::<CharacterStats>(unit).unwrap()
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell_id: u32) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn cancel(h: &mut Harness, unit: Entity, spell: u32) {
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell
    });
    h.advance(2);
}

#[test]
fn augmentation_support_native_prescience_cast_grants_only_recipient_crit() {
    let (mut h, ally) = fixture();
    let owner = h.caster;
    let original = stats(&h, ally).crit_pct;
    let owner_crit = stats(&h, owner).crit_pct;
    h.cast_and_deliver(PRESCIENCE, Some(ally));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), ally, PRESCIENCE_BUFF));
    assert_eq!(stats(&h, ally).crit_pct, original + 3.0);
    assert_eq!(stats(&h, owner).crit_pct, owner_crit);
    // Reapplication of the native child must not accumulate the stat again.
    apply(&mut h, owner, ally, PRESCIENCE_BUFF);
    h.advance(2);
    assert_eq!(stats(&h, ally).crit_pct, original + 3.0);
    cancel(&mut h, ally, PRESCIENCE_BUFF);
    assert_eq!(stats(&h, ally).crit_pct, original);
}

#[test]
fn augmentation_support_native_sands_snapshots_caster_and_updates_on_its_own_clock() {
    let (mut h, ally) = fixture();
    let owner = h.caster;
    let baseline = stats(&h, ally).versatility_pct;
    let owner_versatility = stats(&h, owner).versatility_pct;
    let first_mastery = stats(&h, owner).mastery_pct;
    apply(&mut h, owner, ally, SHIFTING_SANDS);
    h.advance(2);
    assert_eq!(stats(&h, ally).versatility_pct, baseline + first_mastery);
    assert_eq!(stats(&h, owner).versatility_pct, owner_versatility);
    apply(&mut h, owner, owner, ELEMENTAL_BLAST_MASTERY);
    h.advance(2);
    let changed_mastery = stats(&h, owner).mastery_pct;
    assert!(changed_mastery > first_mastery);
    assert_eq!(stats(&h, ally).versatility_pct, baseline + first_mastery);
    h.advance(20);
    assert_eq!(stats(&h, ally).versatility_pct, baseline + changed_mastery);
    cancel(&mut h, owner, ELEMENTAL_BLAST_MASTERY);
    h.advance(20);
    assert_eq!(stats(&h, ally).versatility_pct, baseline + first_mastery);
    cancel(&mut h, ally, SHIFTING_SANDS);
    assert_eq!(stats(&h, ally).versatility_pct, baseline);
}

#[test]
fn augmentation_support_native_sands_uses_each_aura_caster_and_expires_without_refresh() {
    let (mut h, ally) = fixture();
    let owner = h.caster;
    let other_owner = spawn_computed_player(&mut h, 13, 1473, 5.0);
    let other_ally = spawn_computed_player(&mut h, 8, 62, 6.0);
    h.advance(3);
    apply(&mut h, owner, owner, ELEMENTAL_BLAST_MASTERY);
    h.advance(3);
    let boosted = stats(&h, owner).mastery_pct;
    let unboosted = stats(&h, other_owner).mastery_pct;
    assert!(boosted > unboosted);
    let baseline = stats(&h, ally).versatility_pct;
    apply(&mut h, owner, ally, SHIFTING_SANDS);
    apply(&mut h, other_owner, other_ally, SHIFTING_SANDS);
    h.advance(22);
    assert_eq!(stats(&h, ally).versatility_pct, baseline + boosted);
    assert_eq!(stats(&h, other_ally).versatility_pct, baseline + unboosted);
    let duration = world_db()
        .unwrap()
        .store
        .get(SHIFTING_SANDS)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms;
    h.advance(duration as usize / 50 + 2);
    assert!(!has_aura(h.app.world(), ally, SHIFTING_SANDS));
    assert!(!has_aura(h.app.world(), other_ally, SHIFTING_SANDS));
    assert_eq!(stats(&h, ally).versatility_pct, baseline);
    assert_eq!(stats(&h, other_ally).versatility_pct, baseline);
}

#[test]
fn augmentation_support_native_sands_changes_actual_recipient_damage_and_healing() {
    let (mut h, ally) = fixture();
    let owner = h.caster;
    apply(&mut h, owner, ally, SHIFTING_SANDS);
    h.advance(2);
    let recipient = stats(&h, ally);
    assert_eq!(recipient.versatility_pct, stats(&h, owner).mastery_pct);
    // Preserve source coefficients, variance, clocks and ordinary crit rolls.
    crate::spell_cast::cast_triggered(h.app.world_mut(), ally, DAMAGE, Some(h.npc), 1);
    h.advance(60);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|r| r.spell_id == Some(DAMAGE) && r.kind == CombatLogKind::Damage)
        .unwrap();
    let crit = if damage.crit { 2.0 } else { 1.0 };
    let multiplier = 1.0 + recipient.versatility_pct / 100.0;
    assert!(
        varied(recipient.spell_power * 0.59 * multiplier * crit).contains(&(damage.amount as f32))
    );
    h.app.world_mut().get_mut::<Health>(owner).unwrap().current = 1.0;
    crate::spell_cast::cast_triggered(h.app.world_mut(), ally, HEAL, Some(owner), 1);
    h.advance(60);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|r| r.spell_id == Some(HEAL) && r.kind == CombatLogKind::Heal)
        .unwrap();
    let crit = if heal.crit { 2.0 } else { 1.0 };
    assert!(
        varied(recipient.healing_power * 6.82 * multiplier * crit).contains(&(heal.amount as f32))
    );
}
