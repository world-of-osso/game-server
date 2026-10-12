//! Native Feint1966/aura229 damage boundary; amounts from12.1.0.69933.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

const FEINT: u32 = 1_966;
const ARCANE_EXPLOSION: u32 = 1_449;
const FIREBALL: u32 = 133;
const DAMAGE: f32 = 1_000.0;

fn avoidance_harness() -> Harness {
    let mut h = harness(Vec::new(), &[FEINT, ARCANE_EXPLOSION, FIREBALL], 4).unwrap();
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
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 2)
        .unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &spell,
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
