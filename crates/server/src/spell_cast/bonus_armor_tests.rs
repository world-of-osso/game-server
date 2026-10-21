//! TC a352b1fa StatSystem.cpp274: aura466 multiplies the player's final armor.
use super::test_harness::*;
use crate::stats::{PrimaryStat, StatInputs, StatModifiers, compute_character_stats};
use bevy::prelude::*;
use shared::components::{CombatRatings, UnitLevel};

const ARMOR_BUFF: u32 = 161_394;
const SECOND_BUFF: u32 = 262_496;
const ARMOR_DEBUFF: u32 = 238_666;
const NEGATIONS: [u32; 4] = [162_697, 162_698, 162_699, 162_701];
const TEMPERED_SCALES: u32 = 396_571;
const MORTAL_STRIKE: u32 = 12_294;
const ARMOR_CONSTANT: f32 = 210.0;
const DAMAGE: f32 = 1_000.0;

fn bonus_armor_harness() -> Harness {
    let spells: Vec<_> = NEGATIONS
        .into_iter()
        .chain([
            ARMOR_BUFF,
            SECOND_BUFF,
            ARMOR_DEBUFF,
            TEMPERED_SCALES,
            MORTAL_STRIKE,
        ])
        .collect();
    let mut h = harness(Vec::new(), &spells, 1).unwrap();
    let mut constants = vec![0.0; 11];
    constants[10] = ARMOR_CONSTANT;
    h.app
        .insert_resource(crate::level_scaling::ArmorConstants(constants));
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(10));
    h.advance(3);
    h
}

fn apply_native_armor(h: &mut Harness, spell: u32) {
    let effect = world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 466 || effect.aura == 142)
        .unwrap();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: spell,
            effect_mask: 1 << effect.index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn native_factor(spell: u32) -> f32 {
    let effect = world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 466)
        .unwrap();
    1.0 + effect.base_points / 100.0
}

fn calculate_armor(h: &Harness) -> crate::stats::CharacterStats {
    calculate_armor_with_stat_percent(h, 100.0)
}

fn calculate_armor_with_stat_percent(
    h: &Harness,
    stat_percent: f32,
) -> crate::stats::CharacterStats {
    let input = StatInputs {
        class: 8,
        race: 1,
        level: 10,
        primary_stat: Some(PrimaryStat::Intellect),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 0.0,
        attack_power_per_agility: 0.0,
        ranged_attack_power_per_agility: 0.0,
        gear_primary: default(),
        gear_ratings: CombatRatings {
            armor: 100.0,
            ..default()
        },
    };
    let mut mods = h
        .app
        .world()
        .get::<StatModifiers>(h.caster)
        .cloned()
        .unwrap_or_default();
    mods.armor_from_primary_stat_pct = stat_percent;
    compute_character_stats(&input, Some(&mods)).unwrap()
}

fn physical_loss(h: &mut Harness, stats: crate::stats::CharacterStats) -> f32 {
    h.app.world_mut().entity_mut(h.caster).insert(stats);
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: world_db().unwrap().store.get(MORTAL_STRIKE).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(h.caster);
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.caster, DAMAGE, false, false);
    before - h.health(h.caster)
}

#[test]
fn class_completeness_native_bonus_armor_multiplies_base_and_stat_armor() {
    let mut h = bonus_armor_harness();
    apply_native_armor(&mut h, TEMPERED_SCALES);
    let before = calculate_armor(&h);
    apply_native_armor(&mut h, ARMOR_BUFF);
    let after = calculate_armor(&h);
    let base = 100.0
        * (1.0
            + world_db()
                .unwrap()
                .store
                .get(TEMPERED_SCALES)
                .unwrap()
                .effects[0]
                .base_points
                / 100.0);
    let expected = ((base + after.primary.intellect.floor()) * native_factor(ARMOR_BUFF)).floor();
    assert_eq!(after.armor, expected);
    assert!(after.armor > before.armor);
    let dealt = physical_loss(&mut h, after);
    let expected_damage = DAMAGE * ARMOR_CONSTANT / (expected + ARMOR_CONSTANT);
    assert!(
        (dealt - expected_damage).abs() <= 1.0,
        "hit{dealt} expected{expected_damage}"
    );
}

#[test]
fn class_completeness_native_bonus_armor_rounds_only_after_the_final_multiplier() {
    let mut h = bonus_armor_harness();
    apply_native_armor(&mut h, ARMOR_BUFF);
    // Explicit fixture percentage isolates the fractional stat-armor boundary.
    let stat_percent = 75.0;
    let result = calculate_armor_with_stat_percent(&h, stat_percent);
    let scales = world_db()
        .unwrap()
        .store
        .get(TEMPERED_SCALES)
        .unwrap()
        .effects[0]
        .base_points;
    let base = 100.0 * (1.0 + scales / 100.0);
    let expected = ((base + result.primary.intellect * stat_percent / 100.0)
        * native_factor(ARMOR_BUFF))
    .floor();
    assert_eq!(result.armor, expected);
}

#[test]
fn class_completeness_native_bonus_armor_independent_factors_multiply() {
    let mut h = bonus_armor_harness();
    let baseline = calculate_armor(&h);
    apply_native_armor(&mut h, ARMOR_BUFF);
    apply_native_armor(&mut h, SECOND_BUFF);
    apply_native_armor(&mut h, ARMOR_DEBUFF);
    let after = calculate_armor(&h);
    let expected = (baseline.armor
        * native_factor(ARMOR_BUFF)
        * native_factor(SECOND_BUFF)
        * native_factor(ARMOR_DEBUFF))
    .floor();
    assert_eq!(after.armor, expected);
}

#[test]
fn class_completeness_native_bonus_armor_removal_restores_remaining_factor() {
    let mut h = bonus_armor_harness();
    let baseline = calculate_armor(&h).armor;
    apply_native_armor(&mut h, ARMOR_BUFF);
    apply_native_armor(&mut h, SECOND_BUFF);
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ARMOR_BUFF,
    );
    assert_eq!(
        calculate_armor(&h).armor,
        (baseline * native_factor(SECOND_BUFF)).floor()
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SECOND_BUFF,
    );
    assert_eq!(calculate_armor(&h).armor, baseline);
}

#[test]
fn class_completeness_native_bonus_armor_negative_factor_expires_at_native_duration() {
    let mut h = bonus_armor_harness();
    let baseline = calculate_armor(&h).armor;
    apply_native_armor(&mut h, ARMOR_DEBUFF);
    let modified = calculate_armor(&h);
    assert_eq!(
        modified.armor,
        (baseline * native_factor(ARMOR_DEBUFF)).floor()
    );
    let armor = modified.armor;
    let loss = physical_loss(&mut h, modified);
    let expected_damage = DAMAGE * ARMOR_CONSTANT / (armor + ARMOR_CONSTANT);
    assert!(
        (loss - expected_damage).abs() <= 1.0,
        "hit{loss} expected{expected_damage}"
    );
    let native = world_db().unwrap().store.get(ARMOR_DEBUFF).unwrap();
    let duration = native.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    assert!(duration > 0);
    h.advance(duration as usize / 50 + 4);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        ARMOR_DEBUFF
    ));
    assert_eq!(calculate_armor(&h).armor, baseline);
}

#[test]
fn class_completeness_native_bonus_armor_zero_class_fields_do_not_invent_a_bonus() {
    let mut h = bonus_armor_harness();
    let baseline = calculate_armor(&h).armor;
    for spell in NEGATIONS {
        assert_eq!(native_factor(spell), 1.0);
        apply_native_armor(&mut h, spell);
        assert_eq!(calculate_armor(&h).armor, baseline);
    }
}
