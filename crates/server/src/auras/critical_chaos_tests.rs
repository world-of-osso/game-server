//! TC a352b1fa spell_dh.cpp:632-655 spell_dh_critical_chaos: effect0's flat
//! ProcChance modifier on Chaos Strike 197125 is effect1's percent of the owner's
//! crit chance, computed on apply and recalculated by each 1 s effect1 tick.
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

use super::*;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_native_enemy_dodge_reduction, world_db,
};
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const CRITICAL_CHAOS: u32 = 320_413;
const CHAOS_STRIKE: u32 = 197_125;
const CHAOS_STRIKE_PROC_CHANCE: f32 = 20.0;
const CHAOS_STRIKE_INITIAL: u32 = 344_862;
const CHAOS_STRIKE_CAST: u32 = 162_794;
const CHAOS_REFUND: u32 = 193_840;
/// 30% of 350% = 105%: a deterministic modifier endpoint, not a player-build claim.
const GUARANTEED_PROC_CRIT: f32 = 350.0;
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
            source: AuraSource::KnownSpell,
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

#[test]
fn class_completeness_native_critical_chaos_changes_actual_chaos_strike_fury_refund() {
    let mut final_fury = Vec::new();
    for talented in [false, true] {
        // Isolate the scripted modifier from the native 20% random base chance.
        let mut refund = world_db().unwrap().store.get(CHAOS_STRIKE).unwrap().clone();
        refund.aura_options.as_mut().unwrap().proc_chance = 0;
        let mut ids = vec![
            CHAOS_STRIKE_INITIAL,
            CHAOS_STRIKE_CAST,
            222_031,
            199_547,
            CHAOS_REFUND,
        ];
        if talented {
            ids.push(CRITICAL_CHAOS);
        }
        let mut h =
            harness_with_native_enemy_dodge_reduction(vec![refund], &ids, DEMON_HUNTER).unwrap();
        set_crit(&mut h, GUARANTEED_PROC_CRIT);
        let unit = h.caster;
        if talented {
            apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: unit,
                    target: unit,
                    spell_id: CRITICAL_CHAOS,
                    effect_mask: u32::MAX,
                    source: AuraSource::KnownSpell,
                },
            );
        }
        h.set_power(PowerType::Fury, 100.0);
        h.cast_and_deliver(CHAOS_STRIKE_INITIAL, Some(h.npc));
        h.advance(3);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        final_fury.push(h.power(PowerType::Fury));
        let refunds: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(CHAOS_REFUND) && event.kind == CombatLogKind::Energize
            })
            .collect();
        assert_eq!(
            refunds.len(),
            usize::from(talented),
            "one roll, not per weapon child"
        );
        if talented {
            assert_eq!(refunds[0].amount, 20);
        }
    }
    assert_eq!(final_fury[1], final_fury[0] + 20.0);
}
