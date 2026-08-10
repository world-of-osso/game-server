use super::*;
use crate::trade::trade_support::build_trade_party_snapshot;
use shared::trade::{CompletedTrade, TRADE_SLOT_COUNT, TradeItem, TradeOffer};

fn trade_offer(item_guid: u64, count: u16, gold: u32) -> TradeOffer {
    let mut offer = TradeOffer::default();
    offer.slots[0] = Some(TradeItem { item_guid, count });
    offer.gold = gold;
    offer
}

#[test]
fn execute_completed_trade_moves_items_and_gold() {
    let db = persistence::temp_database();
    let (initiator_id, _) =
        persistence::create_character(&db, 1, "TraderOne", 1, 1).expect("initiator");
    let (target_id, _) = persistence::create_character(&db, 2, "TraderTwo", 1, 1).expect("target");

    let initiator_item = persistence::load_inventory(&db, initiator_id).items[0].clone();
    let target_item = persistence::load_inventory(&db, target_id).items[1].clone();
    let initiator_gold_before = persistence::load_inventory(&db, initiator_id).gold;
    let target_gold_before = persistence::load_inventory(&db, target_id).gold;

    let trade = CompletedTrade {
        initiator: initiator_id,
        target: target_id,
        initiator_offer: trade_offer(initiator_item.item_guid, 1, 1_500),
        target_offer: trade_offer(target_item.item_guid, 2, 900),
    };

    let mut app = App::new();
    app.add_plugins(crate::gold::GoldPlugin);
    let initiator_player = app
        .world_mut()
        .spawn((
            Player {
                name: "TraderOne".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            ClientId(initiator_id),
            shared::components::Gold(initiator_gold_before),
        ))
        .id();
    let target_player = app
        .world_mut()
        .spawn((
            Player {
                name: "TraderTwo".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            ClientId(target_id),
            shared::components::Gold(target_gold_before),
        ))
        .id();
    {
        let mut gold_updates = app
            .world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>();
        execute_completed_trade(&db, &trade, &mut gold_updates).expect("trade should complete");
    }
    app.update();

    let initiator_inventory = persistence::load_inventory(&db, initiator_id);
    let target_inventory = persistence::load_inventory(&db, target_id);

    assert_eq!(
        initiator_inventory.gold,
        initiator_gold_before - 1_500 + 900
    );
    assert_eq!(target_inventory.gold, target_gold_before - 900 + 1_500);
    assert_eq!(
        app.world()
            .entity(initiator_player)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        initiator_inventory.gold
    );
    assert_eq!(
        app.world()
            .entity(target_player)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        target_inventory.gold
    );
    assert!(
        initiator_inventory
            .items
            .iter()
            .any(|item| item.item_guid == target_item.item_guid && item.stack_count == 2)
    );
    assert!(
        target_inventory
            .items
            .iter()
            .any(|item| item.item_guid == initiator_item.item_guid && item.stack_count == 1)
    );
}

#[test]
fn execute_completed_trade_fails_when_item_is_missing() {
    let db = persistence::temp_database();
    let (initiator_id, _) =
        persistence::create_character(&db, 1, "TraderOne", 1, 1).expect("initiator");
    let (target_id, _) = persistence::create_character(&db, 2, "TraderTwo", 1, 1).expect("target");

    let trade = CompletedTrade {
        initiator: initiator_id,
        target: target_id,
        initiator_offer: trade_offer(999_999, 1, 0),
        target_offer: TradeOffer::default(),
    };

    let mut gold_updates = crate::gold::GoldUpdateQueue::default();
    let error =
        execute_completed_trade(&db, &trade, &mut gold_updates).expect_err("trade should fail");
    assert!(error.contains("no longer in inventory"));
    assert!(gold_updates.is_empty());
}

#[test]
fn build_trade_party_snapshot_uses_inventory_metadata() {
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character(&db, 1, "TraderOne", 1, 1).expect("character");
    let inventory = persistence::load_inventory(&db, character_id);
    let item = inventory.items[0].clone();
    let snapshot = build_trade_party_snapshot(
        &db,
        character_id,
        "TraderOne".into(),
        true,
        &trade_offer(item.item_guid, 3, 777),
    );

    assert_eq!(snapshot.name, "TraderOne");
    assert!(snapshot.accepted);
    assert_eq!(snapshot.gold, 777);
    assert_eq!(snapshot.slots.len(), TRADE_SLOT_COUNT);
    let slot = snapshot.slots[0]
        .as_ref()
        .expect("slot should be populated");
    assert_eq!(slot.item_guid, item.item_guid);
    assert_eq!(slot.item_id, item.item_id);
    assert_eq!(slot.name, item.name);
    assert_eq!(slot.stack_count, 3);
}
