//! TC a352b1fa spell_monk.cpp spell_monk_power_strike_periodic (TDB1210
//! spell_script_names 121817): each 15 s effect0 PERIODIC_DUMMY tick casts 129914
//! on the owner.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};

const POWER_STRIKE: u32 = 121_817;
const POWER_STRIKE_PROC: u32 = 129_914;
const MONK: u8 = 10;
const TICK_FRAMES: usize = 15_000 / 50 + 2;

fn has_proc(h: &Harness) -> bool {
    has_aura(h.app.world(), h.caster, POWER_STRIKE_PROC)
}

#[test]
fn class_completeness_native_power_strike_casts_proc_each_tick() {
    let ids = [POWER_STRIKE, POWER_STRIKE_PROC];
    let mut h = harness(Vec::new(), &ids, MONK).unwrap();
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: POWER_STRIKE,
            effect_mask: u32::MAX,
            source: AuraSource::KnownSpell,
        },
    );
    assert!(!has_proc(&h), "cast by a tick, not the application hook");
    h.advance(1);
    assert!(
        has_proc(&h),
        "authored ExtraInitialPeriod triggers the first due tick"
    );
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == POWER_STRIKE_PROC
    });
    h.advance(TICK_FRAMES - 10);
    assert!(!has_proc(&h), "next tick waits 15 seconds");
    h.advance(10);
    assert!(has_proc(&h), "every tick recasts the proc");
}
