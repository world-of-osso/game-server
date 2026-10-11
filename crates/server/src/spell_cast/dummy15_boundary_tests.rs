//! Owned/immune/refused boundaries complement batch15 positive casts.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

fn apply(h: &mut Harness, caster: Entity, target: Entity, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
fn dummy15_beacon_reassignment_keeps_one_owned_mark_without_removing_foreign_mark() {
    let mut h = harness(Vec::new(), &[53563, 53651, 53652], 2).unwrap();
    let caster = h.caster;
    let first = spawn_player(&mut h.app, 2, 3.0);
    let second = spawn_player(&mut h.app, 2, 4.0);
    let foreign = spawn_player(&mut h.app, 2, 5.0);
    apply(&mut h, foreign, first, 53563);
    apply(&mut h, caster, first, 53563);
    apply(&mut h, caster, second, 53563);
    let marks = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(first)
        .unwrap();
    assert!(
        !marks
            .iter()
            .any(|a| a.spell_id == 53563 && a.caster == caster)
    );
    assert!(
        marks
            .iter()
            .any(|a| a.spell_id == 53563 && a.caster == foreign)
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(second)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 53563 && a.caster == caster)
    );
}

#[test]
fn dummy15_ranked_rising_sun_immune_hit_does_not_wound() {
    let mut h = harness(Vec::new(), &[107428, 185099, 128595, 115804, 642], 10).unwrap();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, caster, caster, 128595);
    apply(&mut h, target, target, 642);
    h.set_power(PowerType::Chi, 5.0);
    let spell = world_db().unwrap().store.get(107428).unwrap();
    super::execute_cast(h.app.world_mut(), caster, spell, Some(target)).unwrap();
    assert!(!crate::auras::has_aura(h.app.world(), target, 115804));
    assert_eq!(h.health(target), 5000.0);
}

#[test]
fn dummy15_karma_native_expiry_removes_unused_shield() {
    let mut h = harness(Vec::new(), &[122470, 133], 10).unwrap();
    let caster = h.caster;
    let target = h.npc;
    let spell = world_db().unwrap().store.get(122470).unwrap();
    super::execute_cast(h.app.world_mut(), caster, spell, Some(target)).unwrap();
    h.advance(205);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 122470));
    let before = h.app.world().get::<Health>(caster).unwrap().current;
    let damage = world_db().unwrap().store.get(133).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: target,
        spell: damage,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, caster, 100.0, false, false);
    assert_eq!(h.health(caster), before - 100.0);
}
