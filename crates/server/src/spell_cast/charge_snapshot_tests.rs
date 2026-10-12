use super::test_harness::*;

#[test]
fn cooldown_assist_known_charge_spell_publishes_full_category_before_first_cast() {
    let mut h = harness(Vec::new(), &[108853], 8).unwrap();
    // Construction already advances the first in-world frame. Drain its owner
    // messages before the next frame expires unread HostClient deliveries.
    h.inbox
        .charges
        .extend(drain::<shared::protocol::SpellChargesUpdate>(
            &mut h.app,
            h.connection,
        ));
    h.advance(4);
    let update = h
        .inbox
        .charges
        .iter()
        .find(|update| update.category == 1500)
        .unwrap_or_else(|| panic!("initial Fire Blast charge message absent; categories={:?}; history={:?}; received={:?}", h.app.world().resource::<crate::spell_info::SpellStore>().get(108853).unwrap().categories, h.app.world().get::<super::SpellHistory>(h.caster), h.inbox.charges));
    assert_eq!((update.current, update.max, update.remaining_ms), (1, 1, 0));
    assert!(update.recharge_ms > 0);
    assert!(h.inbox.go.is_empty(), "no cast priming was used");
}
