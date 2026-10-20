//! TC a352b1fa SpellAuraEffects.cpp3663: players/pets have base armor; other NPCs do not.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::UnitLevel;

const TEMPERED_SCALES: u32 = 396_571;
const MORTAL_STRIKE: u32 = 12_294;
const METAMORPHOSIS: u32 = 187_827;
const DAMAGE: f32 = 1_000.0;
const ARMOR: f32 = 100.0;
const ARMOR_CONSTANT: f32 = 210.0; // Native ExpectedStat ArmorConstant, level10.

fn armor_harness(pet: bool) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[TEMPERED_SCALES, MORTAL_STRIKE, METAMORPHOSIS],
        1,
    )
    .unwrap();
    let mut constants = vec![0.0; 11];
    constants[10] = ARMOR_CONSTANT;
    h.app
        .insert_resource(crate::level_scaling::ArmorConstants(constants));
    h.app.world_mut().entity_mut(h.caster).insert(UnitLevel(10));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((UnitLevel(10), crate::level_scaling::CreatureArmor(ARMOR)));
    if pet {
        h.app
            .world_mut()
            .entity_mut(h.npc)
            .insert(crate::hunter_pet::HunterPet {
                owner: h.caster,
                slot: 0,
            });
    }
    h.advance(3);
    h
}

fn spell_loss(h: &mut Harness) -> f32 {
    let spell = world_db().unwrap().store.get(MORTAL_STRIKE).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(h.npc);
    super::effects::deal_damage(h.app.world_mut(), &ctx, h.npc, DAMAGE, false, false);
    before - h.health(h.npc)
}

fn apply_meta(h: &mut Harness, target: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: METAMORPHOSIS,
            effect_mask: 1 << 7,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn expected_damage(armor: f32) -> f32 {
    DAMAGE * ARMOR_CONSTANT / (armor + ARMOR_CONSTANT)
}

fn apply_scales(h: &mut Harness) {
    let pet = h.npc;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: pet,
            target: pet,
            spell_id: TEMPERED_SCALES,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn class_completeness_native_base_armor_percent_changes_pet_physical_spell_damage() {
    let mut h = armor_harness(true);
    let baseline = spell_loss(&mut h);
    assert!(
        (baseline - expected_damage(ARMOR)).abs() <= 1.0,
        "baseline{baseline}, expected{}",
        expected_damage(ARMOR)
    );
    apply_scales(&mut h);
    let native = world_db()
        .unwrap()
        .store
        .get(TEMPERED_SCALES)
        .unwrap()
        .effects[0]
        .base_points;
    let expected = expected_damage(ARMOR * (1.0 + native / 100.0));
    let dealt = spell_loss(&mut h);
    assert!(
        (dealt - expected).abs() <= 1.0,
        "pet hit{dealt}, expected{expected}"
    );
}

#[test]
fn class_completeness_native_base_armor_pet_modifier_changes_real_melee_damage() {
    use bevy::ecs::system::RunSystemOnce;
    let mut h = armor_harness(true);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<crate::stats::CharacterStats>()
        .insert(crate::auras::AuraStatMods {
            melee_hit_chance_pct: 100.0,
            ..default()
        });
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::Position {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    let before = h.health(h.npc);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    let baseline = before - h.health(h.npc);
    apply_scales(&mut h);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(h.npc));
    let before = h.health(h.npc);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    let boosted = before - h.health(h.npc);
    assert!(baseline > boosted, "base{baseline} vs protected{boosted}");
    let percent = world_db()
        .unwrap()
        .store
        .get(TEMPERED_SCALES)
        .unwrap()
        .effects[0]
        .base_points;
    let expected = 10.0 * ARMOR_CONSTANT / (ARMOR * (1.0 + percent / 100.0) + ARMOR_CONSTANT);
    assert!(
        (boosted - expected).abs() <= 1.0,
        "melee{boosted} vs{expected}"
    );
}

#[test]
fn class_completeness_native_base_armor_player_factors_multiply_before_stat_armor() {
    let mut h = armor_harness(false);
    let player = h.caster;
    apply_meta(&mut h, player);
    let native = world_db().unwrap();
    let pct_a = native.store.get(TEMPERED_SCALES).unwrap().effects[0].base_points;
    let pct_b = native
        .store
        .get(METAMORPHOSIS)
        .unwrap()
        .effects
        .iter()
        .find(|e| e.aura == 142)
        .unwrap()
        .base_points;
    let mut mods = h
        .app
        .world()
        .get::<crate::stats::StatModifiers>(player)
        .unwrap()
        .clone();
    mods.armor_from_primary_stat_pct = 100.0;
    let input = crate::stats::StatInputs {
        class: 8,
        race: 1,
        level: 10,
        primary_stat: Some(crate::stats::PrimaryStat::Intellect),
        mastery_coefficient: 0.0,
        attack_power_per_strength: 0.0,
        attack_power_per_agility: 0.0,
        ranged_attack_power_per_agility: 0.0,
        gear_primary: default(),
        gear_ratings: shared::components::CombatRatings {
            armor: ARMOR,
            ..default()
        },
    };
    let result = crate::stats::compute_character_stats(&input, Some(&mods)).unwrap();
    let expected = (ARMOR * (1.0 + pct_a / 100.0) * (1.0 + pct_b / 100.0)
        + result.primary.intellect.floor())
    .floor();
    assert_eq!(
        result.armor, expected,
        "stat-derived armor is added after base percentages"
    );
}

#[test]
fn class_completeness_native_base_armor_pet_removal_and_timed_expiry_restore_damage() {
    let mut h = armor_harness(true);
    apply_scales(&mut h);
    let pet = h.npc;
    crate::auras::remove_auras(
        h.app.world_mut(),
        pet,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == TEMPERED_SCALES,
    );
    assert!((spell_loss(&mut h) - expected_damage(ARMOR)).abs() <= 1.0);
    apply_meta(&mut h, pet);
    let native = world_db().unwrap().store.get(METAMORPHOSIS).unwrap();
    let pct = native
        .effects
        .iter()
        .find(|effect| effect.aura == 142)
        .unwrap()
        .base_points;
    assert!((spell_loss(&mut h) - expected_damage(ARMOR * (1.0 + pct / 100.0))).abs() <= 1.0);
    let duration = native.misc.as_ref().unwrap().duration.unwrap().duration_ms;
    assert!(duration > 0);
    h.advance(duration as usize / 50 + 4);
    assert!(!crate::auras::has_aura(h.app.world(), pet, METAMORPHOSIS));
    assert!((spell_loss(&mut h) - expected_damage(ARMOR)).abs() <= 1.0);
}

#[test]
fn class_completeness_native_base_armor_percent_does_not_change_ordinary_npc_armor() {
    let mut h = armor_harness(false);
    apply_scales(&mut h);
    let dealt = spell_loss(&mut h);
    assert!(
        (dealt - expected_damage(ARMOR)).abs() <= 1.0,
        "ordinary NPC{dealt}, expected{}",
        expected_damage(ARMOR)
    );
}
