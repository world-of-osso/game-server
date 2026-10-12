//! DB2 12.1.0.69933:433748s1 protects the learner only during their own8936.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

const PROTECTIVE_GROWTH: u32 = 433_748;
const REGROWTH: u32 = 8_936;
const FIREBALL: u32 = 133;
const INCOMING_DAMAGE: f32 = 1_000.0;

fn growth_harness(learned: bool) -> Harness {
    let mut h = harness(Vec::new(), &[PROTECTIVE_GROWTH, REGROWTH, FIREBALL], 11).unwrap();
    if !learned {
        h.app.world_mut().entity_mut(h.caster).insert(
            crate::class_progression::KnownSpells::fixture(vec![REGROWTH, FIREBALL]),
        );
    }
    h.app.world_mut().entity_mut(h.caster).insert(Health {
        current: 10_000.0,
        max: 10_000.0,
    });
    h.advance(3);
    h
}

fn native_reduction() -> f32 {
    let spell = world_db().unwrap().store.get(PROTECTIVE_GROWTH).unwrap();
    let amount = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    1.0 - amount / 100.0
}

fn cast_regrowth(h: &mut Harness, caster: Entity, target: Entity) {
    super::cast_triggered(h.app.world_mut(), caster, REGROWTH, Some(target), 0);
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), target, REGROWTH));
}

fn spell_loss(h: &mut Harness) -> f32 {
    let spell = world_db().unwrap().store.get(FIREBALL).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(h.caster);
    super::effects::deal_copied_damage(h.app.world_mut(), &ctx, h.caster, INCOMING_DAMAGE);
    before - h.health(h.caster)
}

#[test]
fn class_completeness_native_protective_growth_reduces_damage_only_during_own_regrowth() {
    let mut h = growth_harness(true);
    assert_eq!(spell_loss(&mut h), INCOMING_DAMAGE);
    let caster = h.caster;
    cast_regrowth(&mut h, caster, caster);
    assert_eq!(
        spell_loss(&mut h),
        (INCOMING_DAMAGE * native_reduction()).round()
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == REGROWTH,
    );
    assert_eq!(spell_loss(&mut h), INCOMING_DAMAGE);
}

#[test]
fn class_completeness_native_protective_growth_excludes_foreign_and_other_target_regrowth() {
    let mut h = growth_harness(true);
    let other = spawn_player(&mut h.app, 11, 1.0);
    let caster = h.caster;
    cast_regrowth(&mut h, other, caster);
    assert_eq!(
        spell_loss(&mut h),
        INCOMING_DAMAGE,
        "another Druid's Regrowth is not yours"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == REGROWTH,
    );
    cast_regrowth(&mut h, caster, other);
    assert_eq!(
        spell_loss(&mut h),
        INCOMING_DAMAGE,
        "healing another player does not protect you"
    );
}

#[test]
fn class_completeness_native_protective_growth_ends_at_native_expiry_and_requires_talent() {
    let mut h = growth_harness(false);
    let caster = h.caster;
    cast_regrowth(&mut h, caster, caster);
    assert_eq!(
        spell_loss(&mut h),
        INCOMING_DAMAGE,
        "Regrowth alone grants no mitigation"
    );
    let mut h = growth_harness(true);
    let caster = h.caster;
    cast_regrowth(&mut h, caster, caster);
    assert_eq!(
        spell_loss(&mut h),
        (INCOMING_DAMAGE * native_reduction()).round()
    );
    for _ in 0..1_200 {
        if !crate::auras::has_aura(h.app.world(), caster, REGROWTH) {
            break;
        }
        h.advance(1);
    }
    assert!(
        !crate::auras::has_aura(h.app.world(), caster, REGROWTH),
        "native HoT must expire"
    );
    assert_eq!(spell_loss(&mut h), INCOMING_DAMAGE);
}

fn swing_loss(h: &mut Harness) -> f32 {
    use bevy::ecs::system::RunSystemOnce;
    let target = h.caster;
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<crate::stats::CharacterStats>()
        .remove::<shared::components::Npc>()
        .insert(crate::combat::AutoAttack::new(target));
    let before = h.health(target);
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    before - h.health(target)
}

#[test]
fn class_completeness_native_protective_growth_reduces_real_melee_swings() {
    let mut baseline = growth_harness(true);
    let base_loss = swing_loss(&mut baseline);
    assert!(base_loss > 0.0, "real melee swing must land");
    let mut protected = growth_harness(true);
    let caster = protected.caster;
    cast_regrowth(&mut protected, caster, caster);
    assert_eq!(
        swing_loss(&mut protected),
        (base_loss * native_reduction()).round()
    );
}
