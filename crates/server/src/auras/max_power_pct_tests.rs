//! Native178: TC HandleAuraModMaxPowerPct preserves absolute missing power.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with};
use crate::stats::CharacterStats;
use shared::components::{Mana, PowerType};

const SAVANT: u32 = 190740;
const ARCANE_FAMILIAR: u32 = 210126;
const STABILIZED_ENERGY: u32 = 228451;
const BASE_MANA_BONUS: u32 = 429399;

fn fixture(class: u8) -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[SAVANT, ARCANE_FAMILIAR, STABILIZED_ENERGY, BASE_MANA_BONUS],
        class,
        true,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    let caster = h.caster;
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    h
}

fn apply(h: &mut Harness, target: Entity, spell_id: u32, index: u8) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), target, spell_id));
}

fn remove(h: &mut Harness, spell_id: u32) {
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == spell_id,
    );
}

fn mana(h: &Harness) -> Mana {
    *h.app.world().get::<Mana>(h.caster).unwrap()
}

#[test]
fn powers_missing_catalog_does_not_guess_native_max_power_or_mutate_existing_pool() {
    let mut h = fixture(4);
    let caster = h.caster;
    let before = h
        .app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(caster)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .remove_resource::<crate::spell_cast::powers::PowerTypeTable>()
        .unwrap();
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id: STABILIZED_ENERGY,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), caster, STABILIZED_ENERGY));
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(caster)
            .unwrap(),
        &before
    );
    remove(&mut h, STABILIZED_ENERGY);
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(caster)
            .unwrap(),
        &before
    );
    assert!(
        !h.app
            .world()
            .contains_resource::<crate::spell_cast::powers::PowerTypeTable>()
    );
}

#[test]
fn census_tail_178_native_savant_preserves_missing_mana_and_removes_reversibly() {
    let mut h = fixture(8);
    let caster = h.caster;
    let baseline = mana(&h).max;
    h.app.world_mut().get_mut::<Mana>(caster).unwrap().current = baseline / 2.0;
    apply(&mut h, caster, SAVANT, 0);
    let amount = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SAVANT)
        .unwrap()
        .effects[0]
        .amount;
    assert!(amount > 0.0, "real mastery points must scale nativeSavant");
    let boosted = (baseline * (1.0 + amount / 100.0)).round();
    assert_eq!(
        mana(&h),
        Mana {
            current: baseline / 2.0 + boosted - baseline,
            max: boosted
        }
    );
    h.advance(1);
    assert_eq!(
        mana(&h).max,
        boosted,
        "ordinary stats recompute cannot undo178"
    );
    let before_remove = mana(&h).current;
    remove(&mut h, SAVANT);
    assert_eq!(
        mana(&h),
        Mana {
            current: (before_remove - (boosted - baseline)).max(0.0),
            max: baseline
        }
    );
    h.advance(1);
    assert_eq!(mana(&h).max, baseline);
}

#[test]
fn census_tail_178_native_familiar_composes_with_base_mana_without_regranting_419() {
    let mut h = fixture(8);
    let caster = h.caster;
    let base = h
        .app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .base_mana;
    h.app.world_mut().get_mut::<Mana>(caster).unwrap().current = base / 2.0;
    apply(&mut h, caster, BASE_MANA_BONUS, 1);
    h.advance(3);
    let original = mana(&h);
    assert_eq!(original.max, (base * 1.05).round());
    assert!(
        original.current < base * 0.6,
        "419 retains current instead of granting5%"
    );
    apply(&mut h, caster, ARCANE_FAMILIAR, 0);
    let boosted = (base * 1.05 * 1.1).round();
    assert_eq!(
        mana(&h),
        Mana {
            current: original.current + boosted - original.max,
            max: boosted
        }
    );
    h.advance(1);
    assert_eq!(mana(&h).max, boosted);
    let before = mana(&h).current;
    remove(&mut h, ARCANE_FAMILIAR);
    assert_eq!(
        mana(&h),
        Mana {
            current: before - (boosted - original.max),
            max: original.max
        }
    );
    assert!(has_aura(h.app.world(), caster, BASE_MANA_BONUS));
}

#[test]
fn census_tail_178_native_stabilized_energy_updates_matching_pool_and_removal_clamps() {
    let mut h = fixture(4);
    let caster = h.caster;
    h.set_power(PowerType::Energy, 40.0);
    apply(&mut h, caster, STABILIZED_ENERGY, 1);
    let pool = *h
        .app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(caster)
        .unwrap()
        .get(PowerType::Energy)
        .unwrap();
    assert_eq!((pool.current, pool.max), (45.0, 105.0));
    h.advance(1);
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(caster)
            .unwrap()
            .get(PowerType::Energy)
            .unwrap()
            .max,
        105.0
    );
    h.set_power(PowerType::Energy, 2.0);
    remove(&mut h, STABILIZED_ENERGY);
    assert_eq!(h.power(PowerType::Energy), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(caster)
            .unwrap()
            .get(PowerType::Energy)
            .unwrap()
            .max,
        100.0
    );
}

#[test]
fn census_tail_178_native_npc_does_not_gain_max_or_current_mana() {
    let mut h = fixture(8);
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(Mana {
        current: 500.0,
        max: 1000.0,
    });
    apply(&mut h, npc, ARCANE_FAMILIAR, 0);
    assert_eq!(
        h.app.world().get::<Mana>(npc).unwrap(),
        &Mana {
            current: 500.0,
            max: 1000.0
        }
    );
    h.advance(1);
    assert_eq!(h.app.world().get::<Mana>(npc).unwrap().max, 1000.0);
}
