//! Native70448 aura308; TC a352b1fa Unit.cpp7264–7270.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use crate::spell_cast::test_harness::{Harness, harness, world_db};
use shared::protocol::CombatLogKind;

const VITAL_CLARITY: u32 = 1_266_748;
const RESTORATION_MARKER: u32 = 246_733;
const SPEARHEAD: u32 = 378_957;
const PREDATOR_CLOAK: u32 = 248_212;
const VIVIFY: u32 = 116_670;
const SHEILUN: u32 = 399_491;
const RIPTIDE: u32 = 61_295;
const WILDFIRE_BOMB: u32 = 269_747;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            VITAL_CLARITY,
            RESTORATION_MARKER,
            SPEARHEAD,
            PREDATOR_CLOAK,
            VIVIFY,
            SHEILUN,
            RIPTIDE,
            WILDFIRE_BOMB,
            2061,
            8004,
            19434,
            34026,
            118253,
            5,
            209140,
        ],
        7,
    )
    .unwrap();
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    h
}

fn apply(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let index = if spell_id == SPEARHEAD { 1 } else { 0 };
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

fn chance(h: &Harness, spell_id: u32) -> f32 {
    crit_chance(
        h.app.world(),
        h.caster,
        h.npc,
        world_db().unwrap().store.get(spell_id).unwrap(),
    )
}

#[test]
fn census_tail_308_native_vital_clarity_reads_both_mask_words_and_removes() {
    let mut h = fixture();
    assert_eq!(chance(&h, VIVIFY), 0.0);
    assert_eq!(chance(&h, SHEILUN), 0.0);
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, VITAL_CLARITY, caster, target);
    assert_eq!(chance(&h, VIVIFY), 15.0);
    assert_eq!(chance(&h, SHEILUN), 15.0);
    assert_eq!(chance(&h, 2061), 0.0);
    assert_eq!(chance(&h, RIPTIDE), 0.0);
    remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == VITAL_CLARITY
    });
    assert_eq!(chance(&h, VIVIFY), 0.0);
}

#[test]
fn census_tail_308_native_matching_factors_sum_and_exclude_ranged_and_none() {
    let mut h = fixture();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, SPEARHEAD, caster, target);
    apply(&mut h, PREDATOR_CLOAK, caster, target);
    assert_eq!(chance(&h, WILDFIRE_BOMB), 40.0);
    assert_eq!(chance(&h, 19434), 0.0, "native ranged Aimed Shot");
    assert_eq!(chance(&h, 118253), 0.0, "native ranged Serpent Sting");
    assert_eq!(chance(&h, 34026), 0.0, "native defense0 Kill Command");
    assert_eq!(chance(&h, VIVIFY), 0.0, "unrelated class family");
}

#[test]
fn census_tail_308_native_foreign_caster_and_wrong_recipient_do_not_grant_chance() {
    let mut h = fixture();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, VITAL_CLARITY, target, target);
    apply(&mut h, VITAL_CLARITY, caster, caster);
    assert_eq!(chance(&h, VIVIFY), 0.0);
    remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == VITAL_CLARITY
    });
    apply(&mut h, VITAL_CLARITY, caster, target);
    assert_eq!(chance(&h, VIVIFY), 15.0);
}

#[test]
fn census_tail_308_native_riptide_marker_guarantees_real_heal_not_other_shaman_heals() {
    let mut h = fixture();
    let caster = h.caster;
    // Real native Annihilation800 hit leaves missing health for the critical heal.
    super::super::creature_cast(h.app.world_mut(), h.npc, 209140, Some(caster), true).unwrap();
    h.advance(3);
    let before = h.health(caster);
    assert!(before > 0.0 && before < 1000.0);
    apply(&mut h, RESTORATION_MARKER, caster, caster);
    let spell = world_db().unwrap().store.get(RIPTIDE).unwrap();
    assert_eq!(crit_chance(h.app.world(), caster, caster, spell), 1000.0);
    assert_eq!(
        crit_chance(
            h.app.world(),
            caster,
            caster,
            world_db().unwrap().store.get(8004).unwrap()
        ),
        0.0
    );
    let plan = vec![super::super::PlannedEffect {
        index: spell
            .effects
            .iter()
            .position(|effect| effect.index == 0)
            .unwrap(),
        units: vec![caster],
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    super::super::resolve_effects(h.app.world_mut(), caster, spell, &plan, 0);
    h.advance(3);
    let event = h
        .inbox
        .log
        .iter()
        .rfind(|event| event.spell_id == Some(RIPTIDE) && event.kind == CombatLogKind::Heal)
        .unwrap();
    assert!(event.crit, "{event:?}");
    assert!(
        (1170..=1230).contains(&event.amount),
        "native6SP×100×2crit±2.5%: {event:?}"
    );
    assert_eq!(h.health(caster), 1000.0);
    assert!(event.overflow > 0);
}
