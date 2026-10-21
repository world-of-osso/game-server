//! TC a352b1fa StatSystem.cpp255-283/1000-1005: aura101 physical bit is the armor
//! TOTAL_PCT multiplier for players and creatures; independent factors multiply.
use super::test_harness::*;
use crate::stats::{PrimaryStat, StatInputs, StatModifiers, compute_character_stats};
use bevy::prelude::*;
use shared::components::{CombatRatings, UnitLevel};

/// Protection Warrior passive: aura101 misc1 +6.
const ARMOR_SPECIALIZATION: u32 = 1_234_769;
/// Warrior talent: effect1 aura101 misc1 +5.
const REINFORCED_PLATES: u32 = 382_939;
/// Aura466 +X final armor (bonus_armor_tests).
const ARMOR_BUFF: u32 = 161_394;
const MORTAL_STRIKE: u32 = 12_294;
/// Large enough that every fixture armor stays below the 85% mitigation cap.
const ARMOR_CONSTANT: f32 = 5_000.0;
const DAMAGE: f32 = 1_000.0;
const CREATURE_ARMOR: f32 = 300.0;
/// Near the level-scaled kobold's expected-stat armor (~400), so 6% stays visible.
const CREATURE_ARMOR_CONSTANT: f32 = 500.0;

fn armor_harness() -> Harness {
    armor_harness_with(ARMOR_CONSTANT)
}

fn armor_harness_with(armor_constant: f32) -> Harness {
    let spells = [
        ARMOR_SPECIALIZATION,
        REINFORCED_PLATES,
        ARMOR_BUFF,
        MORTAL_STRIKE,
    ];
    let mut h = harness(Vec::new(), &spells, 1).unwrap();
    // One constant at every attacker level isolates armor from level scaling.
    h.app
        .insert_resource(crate::level_scaling::ArmorConstants(vec![
            armor_constant;
            81
        ]));
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(10));
    h.advance(3);
    h
}

fn native_effect(spell: u32, aura: u32) -> crate::spell_info::SpellEffect {
    world_db()
        .unwrap()
        .store
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == aura)
        .unwrap()
        .clone()
}

fn apply_native(h: &mut Harness, target: Entity, spell: u32, aura: u32) {
    let index = native_effect(spell, aura).index;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: spell,
            effect_mask: 1 << index,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn factor(spell: u32, aura: u32) -> f32 {
    1.0 + native_effect(spell, aura).base_points / 100.0
}

fn calculate_armor(h: &Harness) -> crate::stats::CharacterStats {
    let mods = h
        .app
        .world()
        .get::<StatModifiers>(h.caster)
        .cloned()
        .unwrap_or_default();
    armor_with(mods)
}

/// Gear and stat armor without any aura modifier.
fn unmodified_armor() -> f32 {
    armor_with(StatModifiers::default()).armor
}

fn armor_with(mods: StatModifiers) -> crate::stats::CharacterStats {
    let input = StatInputs {
        class: 1,
        race: 1,
        level: 10,
        primary_stat: Some(PrimaryStat::Strength),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 0.0,
        attack_power_per_agility: 0.0,
        ranged_attack_power_per_agility: 0.0,
        gear_primary: default(),
        gear_ratings: CombatRatings {
            armor: 1_000.0,
            ..default()
        },
    };
    compute_character_stats(&input, Some(&mods)).unwrap()
}

fn physical_loss(h: &mut Harness, source: Entity, target: Entity) -> f32 {
    let ctx = super::effects::EffectCtx {
        caster: source,
        spell: world_db().unwrap().store.get(MORTAL_STRIKE).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(target);
    super::effects::deal_damage(h.app.world_mut(), &ctx, target, DAMAGE, false, false);
    before - h.health(target)
}

fn assert_mitigated(loss: f32, armor: f32) {
    let expected = DAMAGE * ARMOR_CONSTANT / (armor + ARMOR_CONSTANT);
    assert!(
        (loss - expected).abs() <= 1.0,
        "hit{loss} expected{expected}"
    );
}

#[test]
fn class_completeness_native_armor_percent_multiplies_final_player_armor() {
    let mut h = armor_harness();
    let caster = h.caster;
    let baseline = unmodified_armor();
    apply_native(&mut h, caster, ARMOR_SPECIALIZATION, 101);
    apply_native(&mut h, caster, REINFORCED_PLATES, 101);
    apply_native(&mut h, caster, ARMOR_BUFF, 466);
    let stats = calculate_armor(&h);
    let expected = (baseline
        * factor(ARMOR_SPECIALIZATION, 101)
        * factor(REINFORCED_PLATES, 101)
        * factor(ARMOR_BUFF, 466))
    .floor();
    assert_eq!(stats.armor, expected);
    h.app.world_mut().entity_mut(caster).insert(stats);
    let npc = h.npc;
    assert_mitigated(physical_loss(&mut h, npc, caster), expected);
}

#[test]
fn class_completeness_native_armor_percent_removal_restores_remaining_factor() {
    let mut h = armor_harness();
    let caster = h.caster;
    let baseline = unmodified_armor();
    apply_native(&mut h, caster, ARMOR_SPECIALIZATION, 101);
    apply_native(&mut h, caster, REINFORCED_PLATES, 101);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == REINFORCED_PLATES,
    );
    let expected = (baseline * factor(ARMOR_SPECIALIZATION, 101)).floor();
    assert_eq!(calculate_armor(&h).armor, expected);
}

#[test]
fn class_completeness_native_armor_percent_applies_to_creature_armor() {
    let mut h = armor_harness_with(CREATURE_ARMOR_CONSTANT);
    let (caster, npc) = (h.caster, h.npc);
    h.app.world_mut().entity_mut(npc).insert((
        crate::level_scaling::CreatureArmor(CREATURE_ARMOR),
        shared::components::Health {
            current: 100_000.0,
            max: 100_000.0,
        },
    ));
    // GetArmor() includes TOTAL_PCT before GetArmorMultiplierForTarget scales it, so
    // derive the kobold's effective armor from an unmodified hit.
    let baseline = physical_loss(&mut h, caster, npc);
    let armor = DAMAGE * CREATURE_ARMOR_CONSTANT / baseline - CREATURE_ARMOR_CONSTANT;
    apply_native(&mut h, npc, ARMOR_SPECIALIZATION, 101);
    assert!(crate::auras::has_aura(
        h.app.world(),
        npc,
        ARMOR_SPECIALIZATION
    ));
    let expected = DAMAGE * CREATURE_ARMOR_CONSTANT
        / (armor * factor(ARMOR_SPECIALIZATION, 101) + CREATURE_ARMOR_CONSTANT);
    let loss = physical_loss(&mut h, caster, npc);
    // One-point damage truncation in each hit bounds the derived armor.
    assert!(
        (loss - expected).abs() <= 1.5,
        "hit{loss} expected{expected} baseline{baseline}"
    );
    assert!(loss < baseline, "hit{loss} baseline{baseline}");
}
