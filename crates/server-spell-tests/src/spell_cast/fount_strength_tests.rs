//! Native441675: authored max-power fields and max health while own22842 is active.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

const FOUNT_OF_STRENGTH: u32 = 441_675;
const FRENZIED_REGENERATION: u32 = 22_842;

fn fount_harness(learned: bool) -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[FOUNT_OF_STRENGTH, FRENZIED_REGENERATION],
        11,
        true,
    )
    .unwrap();
    if !learned {
        h.app.world_mut().entity_mut(h.caster).insert(
            crate::class_progression::KnownSpells::fixture(vec![FRENZIED_REGENERATION]),
        );
    }
    h.advance(4);
    h
}

fn native_field(index: u8) -> f32 {
    world_db()
        .unwrap()
        .spell_store()
        .get(FOUNT_OF_STRENGTH)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .base_points
}

fn max_health(h: &Harness, unit: Entity) -> f32 {
    h.app.world().get::<Health>(unit).unwrap().max
}

fn apply_regeneration(h: &mut Harness, caster: Entity, target: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: FRENZIED_REGENERATION,
            effect_mask: crate::auras::aura_effect_mask(
                world_db()
                    .unwrap()
                    .spell_store()
                    .get(FRENZIED_REGENERATION)
                    .unwrap(),
            ),
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        FRENZIED_REGENERATION
    ));
}

#[test]
fn class_completeness_native_fount_strength_max_health_tracks_own_regeneration() {
    let mut h = fount_harness(true);
    let caster = h.caster;
    let base = max_health(&h, caster);
    apply_regeneration(&mut h, caster, caster);
    let expected = base * (1.0 + native_field(2) / 100.0);
    assert!(
        (max_health(&h, caster) - expected).abs() <= 1.0,
        "native Fount health: {} versus {expected}",
        max_health(&h, caster)
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == FRENZIED_REGENERATION,
    );
    h.advance(2);
    assert_eq!(max_health(&h, caster), base);
    assert!(
        h.health(caster) <= base,
        "removing max health must preserve a valid current health"
    );
}

#[test]
fn class_completeness_native_fount_strength_excludes_foreign_regeneration_and_absent_talent() {
    let mut h = fount_harness(false);
    let caster = h.caster;
    let base = max_health(&h, caster);
    apply_regeneration(&mut h, caster, caster);
    assert_eq!(max_health(&h, caster), base);
    let mut h = fount_harness(true);
    let caster = h.caster;
    let base = max_health(&h, caster);
    let other = spawn_player(&mut h.app, 11, 1.0);
    h.advance(3);
    apply_regeneration(&mut h, other, caster);
    assert_eq!(
        max_health(&h, caster),
        base,
        "another caster's aura does not activate your talent"
    );
    let before = max_health(&h, other);
    apply_regeneration(&mut h, caster, other);
    assert_eq!(
        max_health(&h, other),
        before,
        "the talent does not protect another recipient"
    );
}

#[test]
fn class_completeness_native_fount_strength_preserves_native_max_powers_and_expires() {
    let baseline = fount_harness(false);
    let mut h = fount_harness(true);
    for (power, field) in [(PowerType::Energy, 0), (PowerType::Rage, 1)] {
        let base = baseline
            .app
            .world()
            .get::<super::powers::PowerPools>(baseline.caster)
            .unwrap()
            .get(power)
            .unwrap()
            .max;
        let active = h
            .app
            .world()
            .get::<super::powers::PowerPools>(h.caster)
            .unwrap()
            .get(power)
            .unwrap()
            .max;
        assert_eq!(active, base + native_field(field));
    }
    let caster = h.caster;
    let base = max_health(&h, caster);
    apply_regeneration(&mut h, caster, caster);
    assert!(max_health(&h, caster) > base);
    for _ in 0..1_200 {
        if !crate::auras::has_aura(h.app.world(), caster, FRENZIED_REGENERATION) {
            break;
        }
        h.advance(1);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        FRENZIED_REGENERATION
    ));
    h.advance(2);
    assert_eq!(max_health(&h, caster), base);
}
