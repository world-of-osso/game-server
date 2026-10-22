//! Native70448 aura50; TC a352b1fa Unit.cpp7331–7342.
use super::test_harness::*;
use crate::auras::{self, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const CRITICAL_POWER: u32 = 462_587;
const VINDICATOR: u32 = 200_376;
const PVP_TEST_BUFF: u32 = 154_429;
const BIO_SPORES: u32 = 154_611;
const UNWIND_INJURY: u32 = 394_806;
const GUARDIAN_SHELL: u32 = 310_443;
const LAY_ON_HANDS: u32 = 633;

fn fixture(critical: bool) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            CRITICAL_POWER,
            VINDICATOR,
            PVP_TEST_BUFF,
            BIO_SPORES,
            UNWIND_INJURY,
            GUARDIAN_SHELL,
            LAY_ON_HANDS,
        ],
        5,
    )
    .unwrap();
    if critical {
        let caster = h.caster;
        apply(&mut h, BIO_SPORES, 2, caster);
    }
    h
}

fn apply(h: &mut Harness, spell_id: u32, index: u8, target: Entity) {
    let caster = h.caster;
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(auras::has_aura(h.app.world(), target, spell_id));
}

fn remove(h: &mut Harness, spell_id: u32) {
    let target = h.caster;
    auras::remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == spell_id
    });
}

fn heal(h: &mut Harness, spell_id: u32) -> (i32, bool) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .unwrap();
    let plan = vec![super::PlannedEffect {
        index,
        units: vec![h.caster],
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    let caster = h.caster;
    super::resolve_effects(h.app.world_mut(), caster, spell, &plan, 0);
    h.advance(3);
    let event = h
        .inbox
        .log
        .iter()
        .rfind(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(spell_id))
        .unwrap();
    (event.amount, event.crit)
}

#[test]
#[ignore = "census-tail50 RED before native critical healing consumer"]
fn census_tail_50_native_critical_healing_multiplies_whole_heal_without_school_filter() {
    let mut h = fixture(true);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (200, true));
    let caster = h.caster;
    apply(&mut h, CRITICAL_POWER, 0, caster);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (230, true));
    remove(&mut h, CRITICAL_POWER);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (200, true));
}

#[test]
#[ignore = "census-tail50 RED before native critical healing consumer"]
fn census_tail_50_native_signed_modifiers_compose_multiplicatively_and_remove() {
    let mut h = fixture(true);
    let caster = h.caster;
    apply(&mut h, VINDICATOR, 0, caster);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (300, true));
    apply(&mut h, PVP_TEST_BUFF, 1, caster);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (225, true));
    remove(&mut h, VINDICATOR);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (150, true));
}

#[test]
#[ignore = "census-tail50 RED before native critical healing consumer"]
fn census_tail_50_native_small_heal_truncates_after_whole_critical_multiplier() {
    let mut h = fixture(true);
    assert_eq!(heal(&mut h, GUARDIAN_SHELL), (10, true));
    let caster = h.caster;
    apply(&mut h, CRITICAL_POWER, 0, caster);
    assert_eq!(heal(&mut h, GUARDIAN_SHELL), (11, true));
}

#[test]
#[ignore = "census-tail50 RED before native critical healing consumer"]
fn census_tail_50_native_noncritical_and_cannot_crit_heals_remain_unchanged() {
    let mut h = fixture(false);
    let caster = h.caster;
    apply(&mut h, VINDICATOR, 0, caster);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (100, false));
    apply(&mut h, BIO_SPORES, 2, caster);
    assert_eq!(heal(&mut h, LAY_ON_HANDS), (1000, false));
}

#[test]
#[ignore = "census-tail50 RED before native critical healing consumer"]
fn census_tail_50_native_another_recipient_cannot_amplify_healers_critical_amount() {
    let mut h = fixture(true);
    let foreign = h.npc;
    apply(&mut h, VINDICATOR, 0, foreign);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (200, true));
    let caster = h.caster;
    apply(&mut h, VINDICATOR, 0, caster);
    assert_eq!(heal(&mut h, UNWIND_INJURY), (300, true));
}
