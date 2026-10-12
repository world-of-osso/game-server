//! TC a352b1fa Object.cpp1725: selected mechanics use min(sum232, strongest negative234).
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

const CHILLED: u32 = 205_708;
const ROOTS: u32 = 339;
const BATTLE_STANCE: u32 = 386_164;
const STRIDE: u32 = 400_804;
const QUICK_FOOTED: u32 = 450_503;
const RELENTLESS: u32 = 196_029;
const SUPERIOR_TACTICS: u32 = 331_582;
const OPPRESSING_ROAR: u32 = 372_048;
const MODIFIERS: [u32; 6] = [
    BATTLE_STANCE,
    STRIDE,
    QUICK_FOOTED,
    RELENTLESS,
    SUPERIOR_TACTICS,
    OPPRESSING_ROAR,
];
const SNARE: i32 = 11;

fn duration_harness(extra: Vec<SpellInfo>) -> Harness {
    let spells: Vec<_> = MODIFIERS.into_iter().chain([CHILLED, ROOTS]).collect();
    let mut h = harness(extra, &spells, 1).unwrap();
    // Passives in the catalog are available to this fixture, not all learned simultaneously.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            CHILLED, ROOTS,
        ]));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| MODIFIERS.contains(&aura.spell_id),
    );
    h.advance(3);
    h
}

fn apply_modifier(h: &mut Harness, spell: u32) {
    let effect = world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| matches!(effect.aura, 232 | 234) && effect.misc_values[0] == SNARE)
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: spell,
            effect_mask: 1 << effect.index,
            source: AuraSource::Cast,
        },
    );
}

fn apply_control(h: &mut Harness, spell: u32) -> u32 {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.npc,
            target: h.caster,
            spell_id: spell,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell)
        .unwrap()
        .duration_ms
}

fn native_percent(spell: u32) -> i32 {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| matches!(effect.aura, 232 | 234) && effect.misc_values[0] == SNARE)
        .unwrap()
        .base_points as i32
}

fn native_duration(spell: u32) -> u32 {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms as u32
}

fn add_percent(duration: u32, percent: i32) -> u32 {
    // TC Util.h: truncate the integer percentage delta before adding it.
    (duration as i64 + (duration as f32 * percent as f32 / 100.0) as i64).max(0) as u32
}

#[test]
fn class_completeness_native_mechanic_duration_shortens_the_receivers_snare() {
    let mut h = duration_harness(Vec::new());
    apply_modifier(&mut h, STRIDE);
    assert_eq!(
        apply_control(&mut h, CHILLED),
        add_percent(native_duration(CHILLED), native_percent(STRIDE))
    );
}

#[test]
fn class_completeness_native_mechanic_duration_232_sums_matching_fields() {
    let mut h = duration_harness(Vec::new());
    for spell in [BATTLE_STANCE, STRIDE, QUICK_FOOTED] {
        apply_modifier(&mut h, spell);
    }
    let percent =
        native_percent(BATTLE_STANCE) + native_percent(STRIDE) + native_percent(QUICK_FOOTED);
    assert_eq!(
        apply_control(&mut h, CHILLED),
        add_percent(native_duration(CHILLED), percent)
    );
}

#[test]
fn class_completeness_native_mechanic_duration_234_selects_strongest_without_stacking() {
    let mut h = duration_harness(Vec::new());
    apply_modifier(&mut h, RELENTLESS);
    apply_modifier(&mut h, SUPERIOR_TACTICS);
    let strongest = native_percent(RELENTLESS).min(native_percent(SUPERIOR_TACTICS));
    assert_eq!(
        apply_control(&mut h, CHILLED),
        add_percent(native_duration(CHILLED), strongest)
    );
}

#[test]
fn class_completeness_native_mechanic_duration_mixed_types_use_the_stronger_rule() {
    let mut h = duration_harness(Vec::new());
    apply_modifier(&mut h, STRIDE);
    apply_modifier(&mut h, BATTLE_STANCE);
    apply_modifier(&mut h, RELENTLESS);
    let stacked = native_percent(STRIDE) + native_percent(BATTLE_STANCE);
    let strongest = stacked.min(native_percent(RELENTLESS));
    assert_eq!(
        apply_control(&mut h, CHILLED),
        add_percent(native_duration(CHILLED), strongest)
    );
}

#[test]
fn class_completeness_native_mechanic_duration_truncates_delta_before_addition() {
    let mut control = world_db().unwrap().store.get(CHILLED).unwrap().clone();
    // Explicit duration fixture exposes integer AddPct, independent of a whole-number native clock.
    let duration = 8_333;
    control
        .misc
        .as_mut()
        .unwrap()
        .duration
        .as_mut()
        .unwrap()
        .duration_ms = duration;
    let mut h = duration_harness(vec![control]);
    apply_modifier(&mut h, BATTLE_STANCE);
    assert_eq!(
        apply_control(&mut h, CHILLED),
        add_percent(duration as u32, native_percent(BATTLE_STANCE))
    );
}
