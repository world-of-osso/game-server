//! TC a352b1fa SpellHistory.cpp:469-500:341 adds its signed milliseconds
//! to both positive cooldowns of spells with the matching category.
use super::test_harness::*;
use shared::spell_data::CastFailReason;

const QUICK_WITTED: u32 = 382_297;
const COUNTERSPELL: u32 = 2139;

fn quick_witted_harness() -> Harness {
    let mut h = harness(Vec::new(), &[QUICK_WITTED, COUNTERSPELL], 8).unwrap();
    h.advance(3);
    h
}

#[test]
fn class_completeness_native_category_cooldown_changes_actual_counterspell_receipt() {
    let mut h = quick_witted_harness();
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let receipt = h
        .inbox
        .cooldowns
        .iter()
        .find(|receipt| receipt.spell_id == COUNTERSPELL && !receipt.is_gcd)
        .expect("Counterspell starts its native category88 cooldown");
    assert_eq!(receipt.duration_ms, 20_000);
    assert_eq!(receipt.category, 88);
}

#[test]
fn class_completeness_native_category_cooldown_allows_real_recast_after_twenty_seconds() {
    let mut h = quick_witted_harness();
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(360);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert_eq!(
        h.inbox.failed.last().unwrap().reason,
        CastFailReason::OnCooldown
    );
    h.inbox.failed.clear();
    h.advance(45);
    h.cast_and_deliver(COUNTERSPELL, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|event| event.spell_id == COUNTERSPELL)
            .count(),
        2
    );
}
