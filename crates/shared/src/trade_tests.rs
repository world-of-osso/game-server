use super::*;

const NEARBY: (f32, f32, f32) = (100.0, 200.0, 50.0);
const ALSO_NEARBY: (f32, f32, f32) = (105.0, 200.0, 50.0);
const FAR_AWAY: (f32, f32, f32) = (500.0, 500.0, 50.0);

// --- Range check ---

#[test]
fn in_range_same_position() {
    assert!(in_trade_range(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
}

#[test]
fn in_range_nearby() {
    assert!(in_trade_range(0.0, 0.0, 0.0, 5.0, 5.0, 0.0));
}

#[test]
fn out_of_range_far() {
    assert!(!in_trade_range(0.0, 0.0, 0.0, 100.0, 100.0, 0.0));
}

#[test]
fn range_boundary() {
    // Exactly at TRADE_RANGE (11.11) should be in range
    assert!(in_trade_range(0.0, 0.0, 0.0, TRADE_RANGE, 0.0, 0.0));
    // Just beyond should be out of range
    assert!(!in_trade_range(0.0, 0.0, 0.0, TRADE_RANGE + 0.01, 0.0, 0.0));
}

// --- Trade initiation ---

#[test]
fn initiate_trade_success() {
    let mut mgr = TradeManager::new();
    let id = mgr
        .initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    assert_eq!(id, 1);
    assert!(mgr.is_trading(100));
    assert!(mgr.is_trading(200));

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.initiator, 100);
    assert_eq!(session.target, 200);
    assert_eq!(session.state, TradeState::Pending);
    assert_eq!(session.created_at, 1000);
}

#[test]
fn cannot_trade_with_self() {
    let mut mgr = TradeManager::new();
    assert_eq!(
        mgr.initiate_trade(100, 100, NEARBY, NEARBY, 1000),
        Err(TradeError::CannotTradeWithSelf)
    );
}

#[test]
fn cannot_trade_out_of_range() {
    let mut mgr = TradeManager::new();
    assert_eq!(
        mgr.initiate_trade(100, 200, NEARBY, FAR_AWAY, 1000),
        Err(TradeError::OutOfRange)
    );
}

#[test]
fn cannot_trade_when_already_trading() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    // Initiator already in a trade
    assert_eq!(
        mgr.initiate_trade(100, 300, NEARBY, ALSO_NEARBY, 1001),
        Err(TradeError::AlreadyTrading)
    );
    // Target already in a trade
    assert_eq!(
        mgr.initiate_trade(300, 200, NEARBY, ALSO_NEARBY, 1001),
        Err(TradeError::AlreadyTrading)
    );
}

// --- Accept/decline ---

#[test]
fn accept_trade_opens_window() {
    let mut mgr = TradeManager::new();
    let id = mgr
        .initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    let accepted_id = mgr.accept_trade(200).unwrap();
    assert_eq!(accepted_id, id);

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.state, TradeState::Open);
}

#[test]
fn only_target_can_accept() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    // Initiator cannot accept their own request
    assert_eq!(mgr.accept_trade(100), Err(TradeError::NoPendingRequest));
}

#[test]
fn accept_nonexistent_fails() {
    let mut mgr = TradeManager::new();
    assert_eq!(mgr.accept_trade(100), Err(TradeError::NoPendingRequest));
}

#[test]
fn decline_removes_session() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    mgr.decline_trade(200).unwrap();
    assert!(!mgr.is_trading(100));
    assert!(!mgr.is_trading(200));
    assert!(mgr.is_empty());
}

#[test]
fn decline_nonexistent_fails() {
    let mut mgr = TradeManager::new();
    assert_eq!(mgr.decline_trade(100), Err(TradeError::NoPendingRequest));
}

// --- Cancel ---

#[test]
fn cancel_by_initiator() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    let other = mgr.cancel_trade(100).unwrap();
    assert_eq!(other, 200);
    assert!(mgr.is_empty());
}

#[test]
fn cancel_by_target() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.accept_trade(200).unwrap();

    let other = mgr.cancel_trade(200).unwrap();
    assert_eq!(other, 100);
    assert!(mgr.is_empty());
}

#[test]
fn cancel_nonexistent_fails() {
    let mut mgr = TradeManager::new();
    assert_eq!(mgr.cancel_trade(100), Err(TradeError::NoActiveSession));
}

#[test]
fn cancel_pending_by_target() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    // Target cancels while still pending (before accepting)
    let other = mgr.cancel_trade(200).unwrap();
    assert_eq!(other, 100);
    assert!(!mgr.is_trading(100));
    assert!(!mgr.is_trading(200));
}

#[test]
fn cancel_discards_offers() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.set_gold(200, 50000).unwrap();

    mgr.cancel_trade(100).unwrap();
    // Both players are free, no session remains
    assert!(mgr.is_empty());
    assert!(mgr.get_session(100).is_none());
}

#[test]
fn cancel_after_one_confirm() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.confirm_trade(100).unwrap();

    // Target cancels after initiator confirmed — trade aborted
    let other = mgr.cancel_trade(200).unwrap();
    assert_eq!(other, 100);
    assert!(mgr.is_empty());
}

// --- Session lookup ---

#[test]
fn session_involves_both_players() {
    let session = TradeSession {
        initiator: 100,
        target: 200,
        state: TradeState::Open,
        created_at: 1000,
        initiator_offer: TradeOffer::default(),
        target_offer: TradeOffer::default(),
        initiator_accepted: false,
        target_accepted: false,
    };
    assert!(session.involves(100));
    assert!(session.involves(200));
    assert!(!session.involves(300));
}

#[test]
fn session_other_player() {
    let session = TradeSession {
        initiator: 100,
        target: 200,
        state: TradeState::Open,
        created_at: 1000,
        initiator_offer: TradeOffer::default(),
        target_offer: TradeOffer::default(),
        initiator_accepted: false,
        target_accepted: false,
    };
    assert_eq!(session.other_player(100), Some(200));
    assert_eq!(session.other_player(200), Some(100));
    assert_eq!(session.other_player(300), None);
}

#[test]
fn get_session_id() {
    let mut mgr = TradeManager::new();
    let id = mgr
        .initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();

    assert_eq!(mgr.get_session_id(100), Some(id));
    assert_eq!(mgr.get_session_id(200), Some(id));
    assert_eq!(mgr.get_session_id(300), None);
}

// --- Cleanup ---

#[test]
fn cleanup_expired_pending() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.initiate_trade(300, 400, NEARBY, ALSO_NEARBY, 2000)
        .unwrap();

    // 30-second timeout: first request expired, second still valid
    mgr.cleanup_expired(1031, 30);
    assert!(!mgr.is_trading(100));
    assert!(!mgr.is_trading(200));
    assert!(mgr.is_trading(300));
    assert_eq!(mgr.len(), 1);
}

#[test]
fn cleanup_does_not_remove_open_sessions() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.accept_trade(200).unwrap();

    // Open session should not be cleaned up even if old
    mgr.cleanup_expired(9999, 30);
    assert!(mgr.is_trading(100));
    assert_eq!(mgr.len(), 1);
}

// --- After cancel, players can trade again ---

#[test]
fn can_retrade_after_cancel() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.cancel_trade(100).unwrap();

    // Both players free to initiate new trades
    mgr.initiate_trade(100, 300, NEARBY, ALSO_NEARBY, 2000)
        .unwrap();
    assert!(mgr.is_trading(100));
    assert!(mgr.is_trading(300));
}

#[test]
fn sequential_session_ids() {
    let mut mgr = TradeManager::new();
    let id1 = mgr
        .initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.cancel_trade(100).unwrap();
    let id2 = mgr
        .initiate_trade(300, 400, NEARBY, ALSO_NEARBY, 2000)
        .unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

// --- TradeOffer ---

fn sword() -> TradeItem {
    TradeItem {
        item_guid: 1001,
        count: 1,
    }
}

fn potion() -> TradeItem {
    TradeItem {
        item_guid: 2001,
        count: 5,
    }
}

#[test]
fn offer_default_is_empty() {
    let offer = TradeOffer::default();
    assert!(offer.is_empty());
    assert_eq!(offer.item_count(), 0);
    assert_eq!(offer.gold, 0);
}

#[test]
fn offer_set_item() {
    let mut offer = TradeOffer::default();
    offer.set_item(0, sword()).unwrap();
    assert_eq!(offer.slots[0], Some(sword()));
    assert_eq!(offer.item_count(), 1);
    assert!(!offer.is_empty());
}

#[test]
fn offer_set_multiple_slots() {
    let mut offer = TradeOffer::default();
    offer.set_item(0, sword()).unwrap();
    offer.set_item(3, potion()).unwrap();
    assert_eq!(offer.item_count(), 2);
    assert_eq!(offer.slots[0], Some(sword()));
    assert_eq!(offer.slots[3], Some(potion()));
}

#[test]
fn offer_replace_item_in_same_slot() {
    let mut offer = TradeOffer::default();
    offer.set_item(0, sword()).unwrap();
    offer.set_item(0, potion()).unwrap();
    assert_eq!(offer.slots[0], Some(potion()));
    assert_eq!(offer.item_count(), 1);
}

#[test]
fn offer_invalid_slot() {
    let mut offer = TradeOffer::default();
    assert_eq!(offer.set_item(6, sword()), Err(TradeError::InvalidSlot));
    assert_eq!(offer.clear_slot(6), Err(TradeError::InvalidSlot));
}

#[test]
fn offer_duplicate_item_rejected() {
    let mut offer = TradeOffer::default();
    offer.set_item(0, sword()).unwrap();
    assert_eq!(
        offer.set_item(1, sword()),
        Err(TradeError::ItemAlreadyOffered)
    );
}

#[test]
fn offer_clear_slot() {
    let mut offer = TradeOffer::default();
    offer.set_item(2, sword()).unwrap();
    offer.clear_slot(2).unwrap();
    assert!(offer.slots[2].is_none());
    assert_eq!(offer.item_count(), 0);
}

#[test]
fn offer_gold_only() {
    let mut offer = TradeOffer::default();
    offer.set_gold(50000);
    assert_eq!(offer.gold, 50000);
    assert!(!offer.is_empty());
    assert_eq!(offer.item_count(), 0);
}

// --- Trade slots via TradeManager ---

fn open_trade() -> TradeManager {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    mgr.accept_trade(200).unwrap();
    mgr
}

#[test]
fn manager_set_item() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.initiator_offer.slots[0], Some(sword()));
    assert!(session.target_offer.is_empty());
}

#[test]
fn manager_set_item_target_side() {
    let mut mgr = open_trade();
    mgr.set_item(200, 2, potion()).unwrap();

    let session = mgr.get_session(200).unwrap();
    assert_eq!(session.target_offer.slots[2], Some(potion()));
    assert!(session.initiator_offer.is_empty());
}

#[test]
fn manager_set_gold() {
    let mut mgr = open_trade();
    mgr.set_gold(100, 100_000).unwrap();

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.initiator_offer.gold, 100_000);
}

#[test]
fn manager_clear_item() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.clear_item(100, 0).unwrap();

    let session = mgr.get_session(100).unwrap();
    assert!(session.initiator_offer.slots[0].is_none());
}

#[test]
fn manager_set_item_pending_fails() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    // Trade is still Pending, not Open
    assert_eq!(mgr.set_item(100, 0, sword()), Err(TradeError::TradeNotOpen));
}

#[test]
fn manager_set_item_no_session_fails() {
    let mut mgr = TradeManager::new();
    assert_eq!(
        mgr.set_item(100, 0, sword()),
        Err(TradeError::NoActiveSession)
    );
}

#[test]
fn both_players_fill_all_slots() {
    let mut mgr = open_trade();
    for slot in 0..TRADE_SLOT_COUNT {
        let item = TradeItem {
            item_guid: 1000 + slot as u64,
            count: 1,
        };
        mgr.set_item(100, slot, item).unwrap();
    }
    for slot in 0..TRADE_SLOT_COUNT {
        let item = TradeItem {
            item_guid: 2000 + slot as u64,
            count: 1,
        };
        mgr.set_item(200, slot, item).unwrap();
    }
    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.initiator_offer.item_count(), 6);
    assert_eq!(session.target_offer.item_count(), 6);
}

#[test]
fn offer_access_by_player() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.set_gold(200, 5000).unwrap();

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.offer(100).unwrap().item_count(), 1);
    assert_eq!(session.offer(200).unwrap().gold, 5000);
    assert!(session.offer(300).is_none());
}

// --- Mutual accept ---

#[test]
fn confirm_one_side_does_not_complete() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();

    let result = mgr.confirm_trade(100).unwrap();
    assert!(result.is_none());

    let session = mgr.get_session(100).unwrap();
    assert!(session.has_accepted(100));
    assert!(!session.has_accepted(200));
    assert!(!session.both_accepted());
}

#[test]
fn confirm_both_sides_completes_trade() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.set_gold(200, 5000).unwrap();

    mgr.confirm_trade(100).unwrap();
    let result = mgr.confirm_trade(200).unwrap();

    let completed = result.unwrap();
    assert_eq!(completed.initiator, 100);
    assert_eq!(completed.target, 200);
    assert_eq!(completed.initiator_offer.slots[0], Some(sword()));
    assert_eq!(completed.target_offer.gold, 5000);

    // Session removed after completion
    assert!(!mgr.is_trading(100));
    assert!(!mgr.is_trading(200));
    assert!(mgr.is_empty());
}

#[test]
fn confirm_empty_trade_completes() {
    let mut mgr = open_trade();
    mgr.confirm_trade(100).unwrap();
    let result = mgr.confirm_trade(200).unwrap();
    assert!(result.is_some());
}

#[test]
fn modify_offer_resets_accepts() {
    let mut mgr = open_trade();
    mgr.confirm_trade(100).unwrap();
    assert!(mgr.get_session(100).unwrap().has_accepted(100));

    // Modifying the offer resets both accept flags
    mgr.set_item(100, 0, sword()).unwrap();
    let session = mgr.get_session(100).unwrap();
    assert!(!session.has_accepted(100));
    assert!(!session.has_accepted(200));
}

#[test]
fn set_gold_resets_accepts() {
    let mut mgr = open_trade();
    mgr.confirm_trade(100).unwrap();
    mgr.confirm_trade(200).unwrap(); // would complete but let's test gold reset
    // Actually that completes... let me structure differently
    let mut mgr = open_trade();
    mgr.confirm_trade(100).unwrap();

    mgr.set_gold(200, 1000).unwrap();
    let session = mgr.get_session(100).unwrap();
    assert!(!session.has_accepted(100));
    assert!(!session.has_accepted(200));
}

#[test]
fn clear_item_resets_accepts() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.confirm_trade(100).unwrap();

    mgr.clear_item(100, 0).unwrap();
    let session = mgr.get_session(100).unwrap();
    assert!(!session.has_accepted(100));
}

#[test]
fn unconfirm_withdraws_accept() {
    let mut mgr = open_trade();
    mgr.confirm_trade(100).unwrap();
    assert!(mgr.get_session(100).unwrap().has_accepted(100));

    mgr.unconfirm_trade(100).unwrap();
    assert!(!mgr.get_session(100).unwrap().has_accepted(100));
}

#[test]
fn unconfirm_without_accept_fails() {
    let mut mgr = open_trade();
    assert_eq!(mgr.unconfirm_trade(100), Err(TradeError::NotConfirmed));
}

#[test]
fn confirm_pending_trade_fails() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000)
        .unwrap();
    assert_eq!(mgr.confirm_trade(100), Err(TradeError::TradeNotOpen));
}

#[test]
fn confirm_no_session_fails() {
    let mut mgr = TradeManager::new();
    assert_eq!(mgr.confirm_trade(100), Err(TradeError::NoActiveSession));
}

#[test]
fn reconfirm_after_modify_then_complete() {
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.confirm_trade(100).unwrap();
    mgr.confirm_trade(200).unwrap(); // would complete, but 200 modifies

    // Wait — both confirmed, so it completed. Let me restructure.
    let mut mgr = open_trade();
    mgr.set_item(100, 0, sword()).unwrap();
    mgr.confirm_trade(100).unwrap();

    // Target modifies their offer — resets accepts
    mgr.set_gold(200, 1000).unwrap();
    assert!(!mgr.get_session(100).unwrap().has_accepted(100));

    // Both re-confirm
    mgr.confirm_trade(100).unwrap();
    let result = mgr.confirm_trade(200).unwrap();
    let completed = result.unwrap();
    assert_eq!(completed.initiator_offer.slots[0], Some(sword()));
    assert_eq!(completed.target_offer.gold, 1000);
}

include!("trade_extended_tests.rs");
