// BoP validation, trade logging, and range check tests
// (extracted from trade_tests.rs)

use crate::item_data::BoundState;

// --- BoP validation ---

#[test]
fn unbound_item_always_tradeable() {
    let result = validate_trade_binding(&BoundState::Unbound, None, 200, 1000);
    assert!(result.is_ok());
}

#[test]
fn bound_item_without_window_rejected() {
    let bound = BoundState::Bound { character_id: 100 };
    assert_eq!(
        validate_trade_binding(&bound, None, 200, 1000),
        Err(TradeError::ItemSoulbound)
    );
}

#[test]
fn bound_item_with_valid_window_accepted() {
    let bound = BoundState::Bound { character_id: 100 };
    let window = BopTradeWindow {
        looted_at: 1000,
        eligible_players: vec![200, 201, 202],
    };
    let result = validate_trade_binding(&bound, Some(&window), 200, 2000);
    assert!(result.is_ok());
}

#[test]
fn bound_item_ineligible_player_rejected() {
    let bound = BoundState::Bound { character_id: 100 };
    let window = BopTradeWindow {
        looted_at: 1000,
        eligible_players: vec![200, 201],
    };
    assert_eq!(
        validate_trade_binding(&bound, Some(&window), 300, 2000),
        Err(TradeError::ItemSoulbound)
    );
}

#[test]
fn bound_item_expired_window_rejected() {
    let bound = BoundState::Bound { character_id: 100 };
    let window = BopTradeWindow {
        looted_at: 1000,
        eligible_players: vec![200],
    };
    let after_window = 1000 + BOP_TRADE_WINDOW_SECS;
    assert_eq!(
        validate_trade_binding(&bound, Some(&window), 200, after_window),
        Err(TradeError::ItemSoulbound)
    );
}

#[test]
fn bop_window_just_before_expiry() {
    let window = BopTradeWindow {
        looted_at: 1000,
        eligible_players: vec![200],
    };
    let just_before = 1000 + BOP_TRADE_WINDOW_SECS - 1;
    assert!(window.is_eligible(200, just_before));
    assert!(!window.is_expired(just_before));
    assert_eq!(window.remaining(just_before), 1);
}

#[test]
fn bop_window_remaining_after_expiry() {
    let window = BopTradeWindow {
        looted_at: 1000,
        eligible_players: vec![200],
    };
    assert_eq!(window.remaining(1000 + BOP_TRADE_WINDOW_SECS + 100), 0);
}

#[test]
fn bop_window_is_expired() {
    let window = BopTradeWindow {
        looted_at: 0,
        eligible_players: vec![200],
    };
    assert!(!window.is_expired(BOP_TRADE_WINDOW_SECS - 1));
    assert!(window.is_expired(BOP_TRADE_WINDOW_SECS));
}

#[test]
fn bop_window_multiple_eligible() {
    let window = BopTradeWindow {
        looted_at: 0,
        eligible_players: vec![200, 201, 202, 203, 204],
    };
    assert!(window.is_eligible(202, 100));
    assert!(!window.is_eligible(999, 100));
}

// --- Trade logging ---

fn sample_trade() -> CompletedTrade {
    let mut init_offer = TradeOffer::default();
    init_offer.set_item(0, sword()).unwrap();
    let mut tgt_offer = TradeOffer::default();
    tgt_offer.set_gold(50000);
    CompletedTrade {
        initiator: 100, target: 200,
        initiator_offer: init_offer, target_offer: tgt_offer,
    }
}

#[test]
fn log_record_and_get() {
    let mut log = TradeLog::new();
    assert!(log.is_empty());

    let id = log.record(&sample_trade(), 5000);
    assert_eq!(id, 1);
    assert_eq!(log.len(), 1);

    let entry = log.get(id).unwrap();
    assert_eq!(entry.initiator, 100);
    assert_eq!(entry.target, 200);
    assert_eq!(entry.completed_at, 5000);
    assert_eq!(entry.initiator_offer.slots[0], Some(sword()));
    assert_eq!(entry.target_offer.gold, 50000);
}

#[test]
fn log_sequential_ids() {
    let mut log = TradeLog::new();
    let id1 = log.record(&sample_trade(), 1000);
    let id2 = log.record(&sample_trade(), 2000);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn log_get_nonexistent() {
    let log = TradeLog::new();
    assert!(log.get(999).is_none());
}

#[test]
fn log_trades_for_player() {
    let mut log = TradeLog::new();
    log.record(&sample_trade(), 1000);

    let mut other = sample_trade();
    other.initiator = 300;
    other.target = 100;
    log.record(&other, 2000);

    let mut unrelated = sample_trade();
    unrelated.initiator = 400;
    unrelated.target = 500;
    log.record(&unrelated, 3000);

    let player_100 = log.trades_for_player(100);
    assert_eq!(player_100.len(), 2);

    let player_500 = log.trades_for_player(500);
    assert_eq!(player_500.len(), 1);

    assert!(log.trades_for_player(999).is_empty());
}

#[test]
fn log_trades_in_range() {
    let mut log = TradeLog::new();
    log.record(&sample_trade(), 1000);
    log.record(&sample_trade(), 2000);
    log.record(&sample_trade(), 3000);

    let range = log.trades_in_range(1500, 2500);
    assert_eq!(range.len(), 1);
    assert_eq!(range[0].completed_at, 2000);

    assert_eq!(log.trades_in_range(0, 5000).len(), 3);
}

#[test]
fn log_drain_before() {
    let mut log = TradeLog::new();
    log.record(&sample_trade(), 1000);
    log.record(&sample_trade(), 2000);
    log.record(&sample_trade(), 3000);

    let old = log.drain_before(2500);
    assert_eq!(old.len(), 2);
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(3).unwrap().completed_at, 3000);
}

#[test]
fn log_drain_none() {
    let mut log = TradeLog::new();
    log.record(&sample_trade(), 1000);

    let old = log.drain_before(500);
    assert!(old.is_empty());
    assert_eq!(log.len(), 1);
}

// --- Range check during trade ---

#[test]
fn check_range_in_range_ok() {
    let mgr = open_trade();
    assert!(mgr.check_range(100, NEARBY, ALSO_NEARBY).is_ok());
}

#[test]
fn check_range_out_of_range() {
    let mgr = open_trade();
    assert_eq!(mgr.check_range(100, NEARBY, FAR_AWAY), Err(TradeError::OutOfRange));
}

#[test]
fn check_range_no_session() {
    let mgr = TradeManager::new();
    assert_eq!(mgr.check_range(100, NEARBY, ALSO_NEARBY), Err(TradeError::NoActiveSession));
}

#[test]
fn enforce_range_cancels_out_of_range() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();
    mgr.accept_trade(200).unwrap();
    mgr.initiate_trade(300, 400, NEARBY, ALSO_NEARBY, 1001).unwrap();
    mgr.accept_trade(400).unwrap();

    let cancelled = mgr.enforce_range(|player| match player {
        100 => Some(NEARBY),
        200 => Some(ALSO_NEARBY),
        300 => Some(NEARBY),
        400 => Some(FAR_AWAY),
        _ => None,
    });

    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0], (300, 400));
    assert!(mgr.is_trading(100));
    assert!(!mgr.is_trading(300));
}

#[test]
fn enforce_range_missing_position_cancels() {
    let mut mgr = open_trade();

    let cancelled = mgr.enforce_range(|player| match player {
        100 => Some(NEARBY),
        _ => None,
    });

    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0], (100, 200));
    assert!(mgr.is_empty());
}

#[test]
fn enforce_range_all_in_range_no_cancel() {
    let mut mgr = open_trade();

    let cancelled = mgr.enforce_range(|player| match player {
        100 => Some(NEARBY),
        200 => Some(ALSO_NEARBY),
        _ => None,
    });

    assert!(cancelled.is_empty());
    assert!(mgr.is_trading(100));
}

#[test]
fn enforce_range_pending_also_checked() {
    let mut mgr = TradeManager::new();
    mgr.initiate_trade(100, 200, NEARBY, ALSO_NEARBY, 1000).unwrap();

    let cancelled = mgr.enforce_range(|player| match player {
        100 => Some(NEARBY),
        200 => Some(FAR_AWAY),
        _ => None,
    });

    assert_eq!(cancelled.len(), 1);
    assert!(mgr.is_empty());
}
