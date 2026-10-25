//! Native Feint1966/aura229 damage boundary; amounts from12.1.0.69933.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

const FEINT: u32 = 1_966;
const ARCANE_EXPLOSION: u32 = 1_449;
const FIREBALL: u32 = 133;
const DAMAGE: f32 = 1_000.0;

fn avoidance_harness() -> Harness {
    let mut h = harness(Vec::new(), &[FEINT, ARCANE_EXPLOSION, FIREBALL, 269], 4).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Health {
        current: 100_000.0,
        max: 100_000.0,
    });
    h.advance(3);
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), caster, FEINT, Some(caster), 0);
    assert!(crate::auras::has_aura(h.app.world(), caster, FEINT));
    h
}

fn native_feint_factor() -> f32 {
    let spell = world_db().unwrap().store.get(FEINT).unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.aura == 229)
        .unwrap();
    1.0 + effect.base_points / 100.0
}

fn damage_loss(h: &mut Harness, id: u32) -> f32 {
    let spell = world_db().unwrap().store.get(id).unwrap().clone();
    damage_loss_with_spell(h, &spell)
}

fn damage_loss_with_spell(h: &mut Harness, spell: &crate::spell_info::SpellInfo) -> f32 {
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 2)
        .unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(h.caster);
    // Use the actual effect handler, with fixed SP-independent damage input.
    let mut native_effect = effect.clone();
    native_effect.base_points = DAMAGE;
    native_effect.bonus_coefficient = 0.0;
    native_effect.coefficient = 0.0;
    native_effect.scaling_class = 0;
    native_effect.variance = 0.0;
    super::effects::handler(2).unwrap()(h.app.world_mut(), &ctx, &native_effect, h.caster);
    before - h.health(h.caster)
}

#[test]
fn class_completeness_native_feint_reduces_area_not_single_target_damage() {
    let mut h = avoidance_harness();
    assert_eq!(damage_loss(&mut h, FIREBALL), DAMAGE);
    assert_eq!(
        damage_loss(&mut h, ARCANE_EXPLOSION),
        DAMAGE * native_feint_factor()
    );
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FEINT,
    );
    assert_eq!(damage_loss(&mut h, ARCANE_EXPLOSION), DAMAGE);
}

fn apply_native(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    crate::auras::apply_aura_with_spell(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: crate::auras::aura_effect_mask(spell),
            source: crate::auras::AuraSource::Cast,
        },
        spell,
    );
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

#[test]
fn class_completeness_native_feint_multiplies_other_avoidance_and_expires() {
    const IMPROVED_WARDING: u32 = 1_297_073;
    let mut h = avoidance_harness();
    let caster = h.caster;
    apply_native(&mut h, IMPROVED_WARDING, caster, caster);
    let warding = world_db().unwrap().store.get(IMPROVED_WARDING).unwrap();
    let amount = warding
        .effects
        .iter()
        .find(|effect| effect.aura == 229)
        .unwrap()
        .base_points;
    let factor = native_feint_factor() * (1.0 + amount / 100.0);
    assert_eq!(
        damage_loss(&mut h, ARCANE_EXPLOSION),
        (DAMAGE * factor).trunc()
    );
    for _ in 0..1_200 {
        if !crate::auras::has_aura(h.app.world(), caster, FEINT) {
            break;
        }
        h.advance(1);
    }
    assert!(!crate::auras::has_aura(h.app.world(), caster, FEINT));
    assert_eq!(
        damage_loss(&mut h, ARCANE_EXPLOSION),
        (DAMAGE * (1.0 + amount / 100.0)).trunc()
    );
}

#[test]
fn class_completeness_native_avoidance_school_mask_excludes_other_school_and_unit() {
    const INOCULATED: u32 = 69_291;
    const SEED_EXPLOSION: u32 = 27_285;
    let mut h = avoidance_harness();
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FEINT,
    );
    apply_native(&mut h, INOCULATED, caster, caster);
    assert_eq!(damage_loss(&mut h, ARCANE_EXPLOSION), DAMAGE);
    assert_eq!(damage_loss(&mut h, SEED_EXPLOSION), 750.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == INOCULATED,
    );
    let npc = h.npc;
    apply_native(&mut h, INOCULATED, npc, npc);
    assert_eq!(damage_loss(&mut h, SEED_EXPLOSION), DAMAGE);
}

#[test]
fn class_completeness_native_feint_uses_damage_effect_not_area_sibling() {
    let mut h = avoidance_harness();
    let mut spell = world_db().unwrap().store.get(FIREBALL).unwrap().clone();
    let area_spell = world_db().unwrap().store.get(ARCANE_EXPLOSION).unwrap();
    let mut area_sibling = area_spell
        .effects
        .iter()
        .find(|effect| effect.effect == 2)
        .unwrap()
        .clone();
    area_sibling.index = 1;
    spell.effects.push(area_sibling);
    assert_eq!(damage_loss_with_spell(&mut h, &spell), DAMAGE);
}

#[test]
fn class_completeness_native_feint_reduces_periodic_area_damage_once() {
    const WALK_OF_FIRE: u32 = 269;
    let mut h = avoidance_harness();
    let caster = h.caster;
    let mut spell = world_db().unwrap().store.get(WALK_OF_FIRE).unwrap().clone();
    spell.effects[0].base_points = DAMAGE;
    crate::auras::apply_aura_with_spell(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: caster,
            spell_id: WALK_OF_FIRE,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: crate::auras::AuraSource::Cast,
        },
        &spell,
    );
    let before = h.health(caster);
    for _ in 0..100 {
        h.advance(1);
        if h.health(caster) < before {
            break;
        }
    }
    assert_eq!(before - h.health(caster), DAMAGE * native_feint_factor());
}
