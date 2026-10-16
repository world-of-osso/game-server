//! Native Radius/op6 at the existing unit-center directed-selector boundary.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::components::Rotation;

const SHOCKWAVE: u32 = 46968;
const STUN: u32 = 132168;
const RUMBLING_EARTH: u32 = 275339;
const DRAGON_BREATH: u32 = 31661;
const DIADEM: u32 = 207547;
const CONE_OF_COLD: u32 = 120;

fn spawn_directed_caster(spell: u32, class: u8) -> Harness {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[
            SHOCKWAVE,
            STUN,
            RUMBLING_EARTH,
            DRAGON_BREATH,
            DIADEM,
            CONE_OF_COLD,
        ],
        class,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        crate::class_progression::KnownSpells::fixture(vec![spell]),
    ));
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, RUMBLING_EARTH | DIADEM),
    );
    h.advance(3);
    h.inbox = Inbox::default();
    h
}

fn apply_modifier(h: &mut Harness, spell_id: u32, unit: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn cast_directed(h: &mut Harness, spell: u32) {
    h.cast_and_deliver(spell, None);
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    assert!(
        h.health(h.npc) < 5000.0,
        "ordinary inner-cone damage remains"
    );
}

#[test]
fn directed_radius_native_rumbling_earth_adds_six_yards_at_unit_center_boundary() {
    let mut h = spawn_directed_caster(SHOCKWAVE, 1);
    let caster = h.caster;
    apply_modifier(&mut h, RUMBLING_EARTH, caster);
    let boundary = spawn_npc(&mut h.app, 16.0);
    let outside = spawn_npc(&mut h.app, 16.1);
    let behind = spawn_npc(&mut h.app, -16.0);
    cast_directed(&mut h, SHOCKWAVE);
    assert!(
        h.health(boundary) < 5000.0,
        "native10+6 radius must reach16"
    );
    assert!(crate::auras::has_aura(h.app.world(), boundary, STUN));
    assert_eq!(h.health(outside), 5000.0);
    assert_eq!(
        h.health(behind),
        5000.0,
        "radius does not bypass the cone angle"
    );
}

#[test]
fn directed_radius_native_mage_diadem_uses_the_same_flat_modifier_consumer() {
    let mut h = spawn_directed_caster(DRAGON_BREATH, 8);
    let caster = h.caster;
    apply_modifier(&mut h, DIADEM, caster);
    let boundary = spawn_npc(&mut h.app, 37.0);
    let outside = spawn_npc(&mut h.app, 37.1);
    cast_directed(&mut h, DRAGON_BREATH);
    assert!(
        h.health(boundary) < 5000.0,
        "native12+25 radius must reach37"
    );
    assert_eq!(h.health(outside), 5000.0);
}

#[test]
fn directed_radius_native_absence_and_removal_keep_the_original_radius() {
    for removed in [false, true] {
        let mut h = spawn_directed_caster(SHOCKWAVE, 1);
        if removed {
            let caster = h.caster;
            apply_modifier(&mut h, RUMBLING_EARTH, caster);
            crate::auras::remove_auras(
                h.app.world_mut(),
                caster,
                RemoveReason::Cancelled,
                |aura| aura.spell_id == RUMBLING_EARTH,
            );
        }
        let outside = spawn_npc(&mut h.app, 11.0);
        cast_directed(&mut h, SHOCKWAVE);
        assert_eq!(h.health(outside), 5000.0, "removed={removed}");
    }
}

#[test]
fn directed_radius_native_foreign_talent_does_not_extend_caster_cone() {
    let mut h = spawn_directed_caster(SHOCKWAVE, 1);
    let foreign = spawn_player(&mut h.app, 1, -3.0);
    apply_modifier(&mut h, RUMBLING_EARTH, foreign);
    let outside = spawn_npc(&mut h.app, 11.0);
    cast_directed(&mut h, SHOCKWAVE);
    assert_eq!(h.health(outside), 5000.0);
}

#[test]
fn directed_radius_native_same_family_wrong_mask_does_not_extend_other_cone() {
    let mut h = spawn_directed_caster(CONE_OF_COLD, 8);
    let caster = h.caster;
    apply_modifier(&mut h, DIADEM, caster);
    let outside = spawn_npc(&mut h.app, 13.0);
    cast_directed(&mut h, CONE_OF_COLD);
    assert_eq!(
        h.health(outside),
        5000.0,
        "Diadem does not match native Cone of Cold flags"
    );
}
