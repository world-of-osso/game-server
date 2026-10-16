//! TC a352b1fa spell_monk.cpp spell_monk_power_strike_periodic (TDB1210
//! spell_script_names 121817): each 15 s effect0 PERIODIC_DUMMY tick casts 129914
//! on the owner.
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

use super::*;
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with_native_enemy_dodge_reduction,
};

const POWER_STRIKE: u32 = 121_817;
const POWER_STRIKE_PROC: u32 = 129_914;
const POWER_STRIKE_ENERGIZE: u32 = 121_283;
const BLACKOUT_KICK: u32 = 100_784;
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

#[test]
fn class_completeness_native_power_strike_proc_adds_one_chi_on_a_melee_ability_once() {
    let mut final_chi = Vec::new();
    for proc_ready in [false, true] {
        let ids = [POWER_STRIKE_PROC, POWER_STRIKE_ENERGIZE, BLACKOUT_KICK];
        let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, MONK).unwrap();
        let unit = h.caster;
        h.set_power(PowerType::Chi, 3.0);
        if proc_ready {
            apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: unit,
                    target: unit,
                    spell_id: POWER_STRIKE_PROC,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
        h.cast_and_deliver(BLACKOUT_KICK, Some(h.npc));
        h.advance(3);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        final_chi.push(h.power(PowerType::Chi));
        let energizes: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(POWER_STRIKE_ENERGIZE)
                    && event.kind == CombatLogKind::Energize
            })
            .collect();
        assert_eq!(energizes.len(), usize::from(proc_ready));
        if proc_ready {
            assert_eq!(energizes[0].amount, 1);
            assert_eq!(energizes[0].target, Some(unit.to_bits()));
        }
        assert!(!has_proc(&h), "one-charge proc consumed by the ability");
        h.advance(30);
        h.cast_and_deliver(BLACKOUT_KICK, Some(h.npc));
        h.advance(3);
        assert_eq!(
            h.inbox
                .log
                .iter()
                .filter(|event| {
                    event.spell_id == Some(POWER_STRIKE_ENERGIZE)
                        && event.kind == CombatLogKind::Energize
                })
                .count(),
            usize::from(proc_ready),
            "spent charge cannot proc twice"
        );
    }
    assert_eq!(final_chi[1], final_chi[0] + 1.0);
}
