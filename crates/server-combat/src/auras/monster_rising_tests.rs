//! TC a352b1fa spell_dh.cpp:1659-1689 spell_dh_monster_rising: each 1 s effect0
//! PERIODIC_DUMMY tick keeps 452550 outside Metamorphosis and removes it inside.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};

const MONSTER_RISING: u32 = 452_414;
const MONSTER_RISING_AGILITY: u32 = 452_550;
const METAMORPHOSIS_HAVOC: u32 = 162_264;
const METAMORPHOSIS_VENGEANCE: u32 = 187_827;
const DEMON_HUNTER: u8 = 12;
const TICK_FRAMES: usize = 1000 / 50 + 2;

fn apply_self(h: &mut Harness, spell_id: u32, effect_mask: u32) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}

fn has_agility(h: &Harness) -> bool {
    has_aura(h.app.world(), h.caster, MONSTER_RISING_AGILITY)
}

#[test]
fn class_completeness_native_monster_rising_applies_outside_metamorphosis() {
    let ids = [MONSTER_RISING, MONSTER_RISING_AGILITY];
    let mut h = harness(Vec::new(), &ids, DEMON_HUNTER).unwrap();
    apply_self(&mut h, MONSTER_RISING, u32::MAX);
    assert!(!has_agility(&h), "applied by the first tick, not on apply");
    h.advance(TICK_FRAMES);
    assert!(has_agility(&h));
}

#[test]
fn class_completeness_native_monster_rising_drops_in_either_metamorphosis() {
    for meta in [METAMORPHOSIS_HAVOC, METAMORPHOSIS_VENGEANCE] {
        let ids = [MONSTER_RISING, MONSTER_RISING_AGILITY, meta];
        let mut h = harness(Vec::new(), &ids, DEMON_HUNTER).unwrap();
        apply_self(&mut h, MONSTER_RISING, u32::MAX);
        h.advance(TICK_FRAMES);
        assert!(has_agility(&h), "{meta}");
        apply_self(&mut h, meta, u32::MAX);
        h.advance(TICK_FRAMES);
        assert!(!has_agility(&h), "{meta}");
        let unit = h.caster;
        remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == meta
        });
        h.advance(TICK_FRAMES);
        assert!(has_agility(&h), "{meta}");
    }
}
