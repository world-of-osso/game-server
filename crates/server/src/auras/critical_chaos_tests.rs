//! TC a352b1fa spell_dh.cpp:632-655 spell_dh_critical_chaos: effect0's flat
//! ProcChance modifier on Chaos Strike 197125 is effect1's percent of the owner's
//! crit chance, computed on apply and recalculated by each 1 s effect1 tick.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const CRITICAL_CHAOS: u32 = 320_413;
const CHAOS_STRIKE: u32 = 197_125;
const CHAOS_STRIKE_PROC_CHANCE: f32 = 20.0;
const DEMON_HUNTER: u8 = 12;
const TICK_FRAMES: usize = 1000 / 50 + 2;

fn set_crit(h: &mut Harness, crit_pct: f32) {
    let unit = h.caster;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(unit)
        .unwrap()
        .crit_pct = crit_pct;
}

fn chaos_strike_proc_chance(h: &Harness) -> f32 {
    let world = h.app.world();
    let spell = world.resource::<SpellStore>().get(CHAOS_STRIKE).unwrap();
    spell_mods::modify(
        world,
        h.caster,
        spell,
        spell_mods::SpellModOp::ProcChance,
        CHAOS_STRIKE_PROC_CHANCE,
    )
}

#[test]
fn class_completeness_native_critical_chaos_scales_chaos_strike_proc_chance_with_crit() {
    let ids = [CRITICAL_CHAOS, CHAOS_STRIKE];
    let mut h = harness(Vec::new(), &ids, DEMON_HUNTER).unwrap();
    set_crit(&mut h, 20.0);
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CRITICAL_CHAOS,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    assert_eq!(
        chaos_strike_proc_chance(&h),
        26.0,
        "30% of 20% crit on apply"
    );
    set_crit(&mut h, 40.0);
    h.advance(TICK_FRAMES);
    assert_eq!(
        chaos_strike_proc_chance(&h),
        32.0,
        "tick recalculates 30% of 40%"
    );
}
