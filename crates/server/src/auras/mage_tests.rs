//! Mage auras on real world.db data, as the level-10 harness mage casts them:
//! Frostbolt's Chilled slow, Polymorph's replicated debuff and Arcane Intellect's
//! Intellect buff. Skipped when data/world.db is absent.

use shared::components::{AuraView, UnitAuras};

use super::*;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with, harness_with_plugins, native_creature_world,
};
use crate::stats::CharacterStats;

const FROSTBOLT: u32 = 116;
/// Frostbolt's TRIGGER_MISSILE spell (SCHOOL_DAMAGE).
const FROSTBOLT_DAMAGE: u32 = 228597;
/// APPLY_AURA MOD_DECREASE_SPEED -50, `SpellDuration` 31 (8 s).
const CHILLED: u32 = 205708;
const POLYMORPH: u32 = 118;
/// APPLY_AURA MOD_TOTAL_STAT_PERCENTAGE +3, MiscValueB 8 (Intellect), `SpellDuration` 42 (60 min).
const ARCANE_INTELLECT: u32 = 1459;
const MAGE: u8 = 8;
/// Frostbolt's 2 s cast plus delivery, in 50 ms frames.
const FROSTBOLT_FRAMES: usize = 3 + 2000 / 50 + 2;
/// Polymorph's 1.7 s cast plus delivery.
const POLYMORPH_FRAMES: usize = 3 + 1700 / 50 + 2;

fn view(h: &Harness, unit: Entity, spell_id: u32) -> Option<AuraView> {
    h.app
        .world()
        .get::<UnitAuras>(unit)?
        .auras
        .iter()
        .find(|view| view.spell_id == spell_id)
        .cloned()
}

#[test]
fn frostbolt_chills_its_target_to_half_run_speed_for_8_seconds() {
    let Some(mut h) = harness(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE, CHILLED], MAGE) else {
        return;
    };
    let health = h.health(h.npc);
    h.cast(FROSTBOLT, Some(h.npc));
    h.advance(FROSTBOLT_FRAMES);
    assert!(h.health(h.npc) < health, "{:?}", h.inbox.failed);
    let chilled = view(&h, h.npc, CHILLED).expect("Chilled replicated on the target");
    assert!(chilled.harmful);
    assert_eq!(chilled.duration_ms, 8000);
    assert_eq!(chilled.caster, Some(h.caster.to_bits()));
    assert_ne!(chilled.flags & AuraView::FLAG_FROM_PLAYER, 0);
    assert_eq!(chilled.flags & AuraView::FLAG_HIDDEN, 0);
    let speed = |h: &Harness| {
        h.app
            .world()
            .get::<AuraStatMods>(h.npc)
            .unwrap()
            .movement_speed
    };
    assert_eq!(speed(&h), 0.5);
    h.advance(8000 / 50 + 2);
    assert!(view(&h, h.npc, CHILLED).is_none(), "Chilled expires");
    assert_eq!(speed(&h), 1.0);
}

#[test]
fn polymorph_replicates_its_debuff_on_the_target_with_the_caster_and_duration() {
    let Some(mut h) = harness_with_plugins(Vec::new(), &[POLYMORPH], MAGE, false, |app| {
        app.insert_resource(native_creature_world(&[6, 16372]));
    }) else {
        return;
    };
    h.cast(POLYMORPH, Some(h.npc));
    h.advance(POLYMORPH_FRAMES);
    let polymorph = view(&h, h.npc, POLYMORPH).expect("Polymorph replicated on the target");
    assert!(polymorph.harmful);
    assert_eq!(polymorph.duration_ms, 60_000);
    assert!(
        (59_000..=60_000).contains(&polymorph.remaining_ms),
        "{polymorph:?}"
    );
    assert_eq!(polymorph.caster, Some(h.caster.to_bits()));
    assert_ne!(polymorph.flags & AuraView::FLAG_FROM_PLAYER, 0);
    assert_eq!(polymorph.flags & AuraView::FLAG_HIDDEN, 0);
}

#[test]
fn arcane_intellect_on_a_hostile_target_buffs_the_mage_with_3_percent_intellect() {
    let Some(mut h) = harness_with(Vec::new(), &[ARCANE_INTELLECT], MAGE, true) else {
        return;
    };
    h.advance(2);
    let intellect = |h: &Harness| {
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .primary
            .intellect
    };
    let before = intellect(&h);
    // The hostile kobold is selected: an ally-or-raid spell falls back to the caster.
    h.cast(ARCANE_INTELLECT, Some(h.npc));
    h.advance(5);
    let buff = view(&h, h.caster, ARCANE_INTELLECT).expect("Arcane Intellect on the mage");
    assert!(!buff.harmful);
    assert_eq!(buff.duration_ms, 3_600_000);
    assert!(view(&h, h.npc, ARCANE_INTELLECT).is_none());
    assert!(
        (intellect(&h) - before * 1.03).abs() < 1e-3,
        "{before} → {}",
        intellect(&h)
    );
}

/// `FactionTemplate` 7 "Creature": the Northshire Blackrock Spy's, neutral to players.
const NEUTRAL_CREATURE: u32 = 7;

#[test]
fn polymorph_is_a_debuff_on_a_neutral_creature_by_its_negative_effects() {
    let Some(mut h) = harness_with_plugins(Vec::new(), &[POLYMORPH], MAGE, false, |app| {
        app.insert_resource(native_creature_world(&[6, 16372]));
    }) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::UnitFactionTemplate(NEUTRAL_CREATURE));
    assert!(!crate::spell_cast::targets::is_hostile(
        h.app.world(),
        h.caster,
        h.npc
    ));
    h.cast(POLYMORPH, Some(h.npc));
    h.advance(POLYMORPH_FRAMES);
    let polymorph = view(&h, h.npc, POLYMORPH).expect("Polymorph on the neutral creature");
    assert!(polymorph.harmful, "{polymorph:?}");
}

#[test]
fn effect_positivity_follows_trinitycore() {
    let Some(data) = crate::spell_cast::test_harness::world_db() else {
        return;
    };
    let negative = |id: u32| {
        crate::spell_info::positivity::negative_effects(
            &data.store,
            data.store.get(id).expect("spell"),
        )
    };
    // MOD_CONFUSE and TRANSFORM on an enemy target; MOD_SCALE stays positive.
    assert_eq!(negative(POLYMORPH), 0b011);
    assert_eq!(negative(CHILLED), 0b1);
    assert_eq!(negative(ARCANE_INTELLECT), 0);
    // Shadow Word: Pain PERIODIC_DAMAGE; Power Word: Fortitude +Stamina.
    assert_eq!(negative(589) & 1, 1);
    assert_eq!(negative(21562), 0);
}

#[test]
fn arcane_intellect_cast_on_a_friendly_player_stays_a_buff() {
    let Some(mut h) = harness(Vec::new(), &[ARCANE_INTELLECT], MAGE) else {
        return;
    };
    let ally = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 2.0);
    h.advance(1);
    h.cast(ARCANE_INTELLECT, Some(ally));
    h.advance(5);
    let buff = view(&h, ally, ARCANE_INTELLECT).expect("Arcane Intellect on the ally");
    assert!(!buff.harmful, "{buff:?}");
    assert_eq!(buff.caster, Some(h.caster.to_bits()));
}
