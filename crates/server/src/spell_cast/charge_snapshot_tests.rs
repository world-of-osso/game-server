use super::test_harness::*;

#[test]
fn cooldown_assist_known_charge_spell_publishes_full_category_before_first_cast() {
    let mut h = harness(Vec::new(), &[108853], 8).unwrap();
    // HostClient owner messages need three frames to cross the real loopback.
    h.advance(4);
    let update = h
        .inbox
        .charges
        .iter()
        .find(|update| update.category == 1500)
        .expect("owner receives initial Fire Blast charges without casting");
    assert_eq!((update.current, update.max, update.remaining_ms), (2, 2, 0));
    assert!(update.recharge_ms > 0);
    assert!(h.inbox.go.is_empty(), "no cast priming was used");
}
