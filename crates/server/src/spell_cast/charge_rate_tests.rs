//! Native148: TC a352b1fa SpellHistory.cpp880-914,1003-1005.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, apply_aura};
use shared::components::{Health, Position};
use shared::protocol::{CombatLogKind, SpellChargesUpdate};

const TRUESHOT: u32 = 288_613;
const AIMED_SHOT: u32 = 19_434;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[TRUESHOT, AIMED_SHOT], 3).unwrap();
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 10.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 20_000.0,
        max: 20_000.0,
    };
    h
}

fn apply_charge_rate(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TRUESHOT,
            // Isolate authentic effect1:148, amount40, charge category1715.
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
}

fn cast_aimed_shot(h: &mut Harness) -> SpellChargesUpdate {
    h.inbox.charges.clear();
    h.cast_and_deliver(AIMED_SHOT, Some(h.npc));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(AIMED_SHOT) && event.kind == CombatLogKind::Damage
    }));
    h.inbox
        .charges
        .first()
        .expect("actual consumed charge receipt")
        .clone()
}

#[test]
fn class_completeness_native_charge_rate_trueshot_accelerates_actual_aimed_shot_recharge() {
    let mut base = fixture();
    let normal = cast_aimed_shot(&mut base);
    let mut h = fixture();
    apply_charge_rate(&mut h);
    let fast = cast_aimed_shot(&mut h);
    assert_eq!(fast.category, 1715);
    assert_eq!(fast.current, fast.max - 1);
    assert_eq!(
        fast.recharge_ms,
        (normal.recharge_ms as f64 / 1.4).floor() as u32
    );
}

#[test]
fn class_completeness_native_charge_rate_retimes_running_aimed_shot_and_notifies_owner() {
    let mut h = fixture();
    cast_aimed_shot(&mut h);
    let before = h.inbox.charges.last().unwrap().clone();
    h.inbox.charges.clear();
    apply_charge_rate(&mut h);
    h.advance(3);
    let after = h.inbox.charges.last().expect("owner charge update");
    let expected = (before.remaining_ms as f64 / 1.4).floor() as u32;
    assert!(
        after.remaining_ms.abs_diff(expected) <= 200,
        "{before:?} -> {after:?}"
    );
    assert_eq!(
        after.recharge_ms,
        (before.recharge_ms as f64 / 1.4).floor() as u32
    );
}
