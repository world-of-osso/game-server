//! TC a352b1fa spell_dh.cpp:1086-1140: 452410 ticks keep 453314 outside
//! Metamorphosis; 453314's player spell script keeps effects 0/1 for Havoc and
//! 2/3 for Devourer by primary specialization.
use shared::components::ActiveSpec;

use super::*;
use crate::spell_cast::test_harness::{Harness, harness};

const ENDURING_TORMENT: u32 = 452_410;
const ENDURING_TORMENT_BUFF: u32 = 453_314;
const METAMORPHOSIS_HAVOC: u32 = 162_264;
const METAMORPHOSIS_DEVOURER: u32 = 1_217_607;
const SPEC_HAVOC: u32 = 577;
const SPEC_DEVOURER: u32 = 1480;
const DEMON_HUNTER: u8 = 12;
const TICK_FRAMES: usize = 1000 / 50 + 2;

fn torment_harness(spec: u32, meta: u32) -> Harness {
    let ids = [ENDURING_TORMENT, ENDURING_TORMENT_BUFF, meta];
    let mut h = harness(Vec::new(), &ids, DEMON_HUNTER).unwrap();
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(ActiveSpec(spec));
    apply_self(&mut h, ENDURING_TORMENT);
    h
}

fn apply_self(h: &mut Harness, spell_id: u32) {
    let unit = h.caster;
    apply_aura(
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

/// Applied effect indices of the buff, or None without it.
fn buff_effects(h: &Harness) -> Option<Vec<u8>> {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)?
        .iter()
        .find(|aura| aura.spell_id == ENDURING_TORMENT_BUFF)
        .map(|aura| aura.effects.iter().map(|effect| effect.index).collect())
}

#[test]
fn class_completeness_native_enduring_torment_buff_effects_follow_specialization() {
    for (spec, meta, effects) in [
        (SPEC_HAVOC, METAMORPHOSIS_HAVOC, vec![0, 1]),
        (SPEC_DEVOURER, METAMORPHOSIS_DEVOURER, vec![2, 3]),
    ] {
        let mut h = torment_harness(spec, meta);
        h.advance(TICK_FRAMES);
        assert_eq!(buff_effects(&h), Some(effects), "{spec}");
    }
}

#[test]
fn class_completeness_native_enduring_torment_drops_in_metamorphosis() {
    for (spec, meta) in [
        (SPEC_HAVOC, METAMORPHOSIS_HAVOC),
        (SPEC_DEVOURER, METAMORPHOSIS_DEVOURER),
    ] {
        let mut h = torment_harness(spec, meta);
        h.advance(TICK_FRAMES);
        assert!(buff_effects(&h).is_some(), "{spec}");
        apply_self(&mut h, meta);
        h.advance(TICK_FRAMES);
        assert_eq!(buff_effects(&h), None, "{spec}");
        let unit = h.caster;
        remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == meta
        });
        h.advance(TICK_FRAMES);
        assert!(buff_effects(&h).is_some(), "{spec}");
    }
}
