//! TC a352b1fa SpellHistory.cpp432-455/608-628: native286 scales
//! matching cooldowns and their remaining time when the aura enters/leaves.
use super::test_harness::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TRUESHOT: u32 = 288_613;
const RAPID_FIRE: u32 = 257_044;

fn fixture() -> Harness {
    harness(Vec::new(), &[TRUESHOT, RAPID_FIRE, 257_045, 263_585], 3).unwrap()
}

fn apply_trueshot(h: &mut Harness) {
    let unit = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: TRUESHOT,
            // Only native286 isolates recovery; other Trueshot mechanics have
            // independent source/proof obligations.
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn cast_rapid_fire(h: &mut Harness) {
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    h.advance(50);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|r| r.spell_id == Some(257_045) && r.kind == CombatLogKind::Damage)
    );
}

#[test]
fn class_completeness_native_recovery_rate_trueshot_starts_real_ten_second_rapid_fire_cooldown() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    let cd = h
        .inbox
        .cooldowns
        .iter()
        .find(|r| r.spell_id == RAPID_FIRE && !r.is_gcd)
        .unwrap();
    // Native60% faster means16000×100/160=10000ms, not40% of16000.
    assert_eq!(cd.duration_ms, 10_000);
}

#[test]
fn class_completeness_native_recovery_rate_application_accelerates_an_existing_real_lockout() {
    let mut h = fixture();
    cast_rapid_fire(&mut h);
    // The channel has ended; accelerate only the remaining timer, not a
    // fresh full16s. Recast stays blocked6s later and succeeds9.5s later.
    apply_trueshot(&mut h);
    h.advance(120);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(70);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_recovery_rate_removal_slows_only_remaining_lockout() {
    let mut h = fixture();
    apply_trueshot(&mut h);
    cast_rapid_fire(&mut h);
    let unit = h.caster;
    remove_auras(h.app.world_mut(), unit, RemoveReason::Cancelled, |aura| {
        aura.spell_id == TRUESHOT
    });
    // Undo the rate on the remaining timer only. It stays blocked10.5s
    // later and succeeds12.25s later; restarting16s would still reject.
    h.advance(210);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(35);
    h.cast_and_deliver(RAPID_FIRE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}
