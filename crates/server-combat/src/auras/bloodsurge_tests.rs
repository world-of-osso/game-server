//! TC a352b1fa spell_warrior.cpp:282-316 spell_warr_bloodsurge: each 3 s effect0
//! PERIODIC_DUMMY tick energizes 384362 with chance sqrt(owned Rend auras) × amount.
use shared::components::PowerType;

use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc};

const BLOODSURGE: u32 = 384_361;
const BLOODSURGE_ENERGIZE: u32 = 384_362;
const REND_AURA: u32 = 388_539;
const WARRIOR: u8 = 1;
/// sqrt(12) × 30 = 103.9%: every tick procs, without seeding randomness.
const CERTAIN_RENDS: usize = 12;

fn bloodsurge_harness() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[BLOODSURGE, BLOODSURGE_ENERGIZE, REND_AURA],
        WARRIOR,
    )
    .unwrap();
    let warrior = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: warrior,
            target: warrior,
            spell_id: BLOODSURGE,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h
}

fn rend(h: &mut Harness, caster: Entity, count: usize) {
    for index in 0..count {
        let target = spawn_npc(&mut h.app, 2.0 + index as f32);
        apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster,
                target,
                spell_id: REND_AURA,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

/// Raw Rage gained over the first periodic tick.
fn rage_after_tick(h: &mut Harness) -> f32 {
    h.set_power(PowerType::Rage, 0.0);
    h.advance(3000 / 50 + 2);
    h.power(PowerType::Rage)
}

#[test]
fn class_completeness_native_bloodsurge_energizes_with_enough_owned_rends() {
    let mut h = bloodsurge_harness();
    let warrior = h.caster;
    rend(&mut h, warrior, CERTAIN_RENDS);
    assert!(rage_after_tick(&mut h) >= 50.0);
}

#[test]
fn class_completeness_native_bloodsurge_needs_rends_owned_by_the_warrior() {
    for owned in [false, true] {
        let mut h = bloodsurge_harness();
        if owned {
            assert_eq!(rage_after_tick(&mut h), 0.0, "no Rend means zero chance");
        } else {
            let other = spawn_npc(&mut h.app, 1.0);
            rend(&mut h, other, CERTAIN_RENDS);
            assert_eq!(rage_after_tick(&mut h), 0.0, "foreign Rends do not count");
        }
    }
}
