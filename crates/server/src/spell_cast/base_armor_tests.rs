//! TC a352b1fa SpellAuraEffects.cpp3663: players/pets have base armor; other NPCs do not.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::UnitLevel;

const TEMPERED_SCALES: u32 = 396_571;
const MORTAL_STRIKE: u32 = 12_294;
const DAMAGE: f32 = 1_000.0;
const ARMOR: f32 = 100.0;
const ARMOR_CONSTANT: f32 = 210.0; // Native ExpectedStat ArmorConstant, level10.

fn armor_harness(pet: bool) -> Harness {
    let mut h = harness(Vec::new(), &[TEMPERED_SCALES, MORTAL_STRIKE], 1).unwrap();
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
