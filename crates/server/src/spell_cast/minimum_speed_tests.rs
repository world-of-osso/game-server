//! TC a352b1fa Unit.cpp8931: strongest positive aura305 floors speed after slows.
use super::test_harness::*;
use bevy::prelude::*;

const GHOST_WOLF: u32 = 2_645;
const CHILLED: u32 = 205_708;
const CHI_TORPEDO: u32 = 115_008;
const ROOTS: u32 = 339;
const STUN: u32 = 853;

fn speed_harness() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[GHOST_WOLF, CHILLED, CHI_TORPEDO, ROOTS, STUN],
        7,
    )
    .unwrap();
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), caster, GHOST_WOLF, Some(caster), 0);
    super::cast_triggered(h.app.world_mut(), h.npc, CHILLED, Some(caster), 0);
    h.advance(3);
    h
}

fn apply_torpedo_speed(h: &mut Harness, target: Entity) {
    // Only native floor/normalization effects: unrelated dash/child mechanics are not tested.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: CHI_TORPEDO,
            effect_mask: (1 << 2) | (1 << 4),
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn speeds(h: &Harness) -> [f32; 3] {
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    [mods.movement_speed, mods.flight_speed, mods.swim_speed]
}

#[test]
fn class_completeness_native_minimum_speed_ghost_wolf_restores_base_after_slow() {
    let h = speed_harness();
    let effect = world_db()
        .unwrap()
        .store
        .get(GHOST_WOLF)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 305)
        .unwrap();
    assert_eq!(speeds(&h), [effect.base_points / 100.0; 3]);
}

#[test]
fn class_completeness_native_minimum_speed_uses_strongest_floor_after_normalization() {
    let mut h = speed_harness();
    let caster = h.caster;
    apply_torpedo_speed(&mut h, caster);
    let native = world_db()
        .unwrap()
        .store
        .get(CHI_TORPEDO)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura == 305)
        .unwrap()
        .base_points;
    assert_eq!(speeds(&h), [native / 100.0; 3]);
}

#[test]
fn class_completeness_native_minimum_speed_expiry_restores_weaker_floor() {
    let mut h = speed_harness();
    let caster = h.caster;
    apply_torpedo_speed(&mut h, caster);
    assert_eq!(speeds(&h), [3.0; 3]);
    let duration = world_db()
        .unwrap()
        .store
        .get(CHI_TORPEDO)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .unwrap()
        .duration_ms;
    h.advance((duration / 50) as usize + 2);
    assert!(!crate::auras::has_aura(h.app.world(), caster, CHI_TORPEDO));
    assert_eq!(speeds(&h), [1.0; 3]);
}

#[test]
fn class_completeness_native_minimum_speed_never_lowers_faster_movement() {
    let mut h = speed_harness();
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == CHILLED,
    );
    // TC a352b1fa Unit.cpp:8818–8829 separates run auras from swim aura58.
    // Ghost Wolf's aura129 grants +30% run only; aura305 floors swim at 100%
    // after slows (Unit.cpp:8931–8939), not at the run multiplier.
    assert_eq!(speeds(&h), [1.3, 1.0, 1.0]);
    let npc = h.npc;
    apply_torpedo_speed(&mut h, npc);
    assert_eq!(speeds(&h), [1.3, 1.0, 1.0]);
}

#[test]
fn class_completeness_native_minimum_speed_stun_blocks_floor_until_removed() {
    let mut h = speed_harness();
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), h.npc, STUN, Some(caster), 0);
    h.advance(3);
    assert_eq!(speeds(&h), [0.0; 3]);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == STUN,
    );
    assert_eq!(speeds(&h), [1.0; 3]);
}

#[test]
fn class_completeness_native_minimum_speed_does_not_override_root_or_persist_after_removal() {
    let mut h = speed_harness();
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), h.npc, ROOTS, Some(caster), 0);
    h.advance(3);
    assert_eq!(speeds(&h), [0.0; 3]);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == ROOTS || aura.spell_id == GHOST_WOLF,
    );
    let slow = world_db().unwrap().store.get(CHILLED).unwrap().effects[0].base_points;
    assert_eq!(speeds(&h), [1.0 + slow / 100.0; 3]);
}
