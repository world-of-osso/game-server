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
    let id = mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

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
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

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
    let id = mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

    let accepted_id = mgr.accept_trade(200).unwrap();
    assert_eq!(accepted_id, id);

    let session = mgr.get_session(100).unwrap();
    assert_eq!(session.state, TradeState::Open);
}

#[test]
fn only_target_can_accept() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

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
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

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
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

    let other = mgr.cancel_trade(100).unwrap();
    assert_eq!(other, 200);
    assert!(mgr.is_empty());
}

#[test]
fn cancel_by_target() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
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

// --- Session lookup ---

#[test]
fn session_involves_both_players() {
    let session = TradeSession {
        initiator: 100, target: 200,
        state: TradeState::Open, created_at: 1000,
    };
    assert!(session.involves(100));
    assert!(session.involves(200));
    assert!(!session.involves(300));
}

#[test]
fn session_other_player() {
    let session = TradeSession {
        initiator: 100, target: 200,
        state: TradeState::Open, created_at: 1000,
    };
    assert_eq!(session.other_player(100), Some(200));
    assert_eq!(session.other_player(200), Some(100));
    assert_eq!(session.other_player(300), None);
}

#[test]
fn get_session_id() {
    let mut mgr = TradeManager::new();
    let id = mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

    assert_eq!(mgr.get_session_id(100), Some(id));
    assert_eq!(mgr.get_session_id(200), Some(id));
    assert_eq!(mgr.get_session_id(300), None);
}

// --- Cleanup ---

#[test]
fn cleanup_expired_pending() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
    mgr.initiate_trade(300, 400, NEARBY, ALSO_NEARBY, 2000).unwrap();

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
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
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
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
    mgr.cancel_trade(100).unwrap();

    // Both players free to initiate new trades
    mgr.initiate_trade(100, 300, NEARBY, ALSO_NEARBY, 2000).unwrap();
    assert!(mgr.is_trading(100));
    assert!(mgr.is_trading(300));
}

#[test]
fn sequential_session_ids() {
    let mut mgr = TradeManager::new();
    let id1 = mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
    mgr.cancel_trade(100).unwrap();
    let id2 = mgr.initiate_trade(300, 400, NEARBY, ALSO_NEARBY, 2000).unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}
