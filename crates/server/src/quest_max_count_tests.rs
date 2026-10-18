//! In-memory quest reward/starter fixtures exercise real owner requests and persistence.
use super::*;
use crate::bank_storage::BankTabData;
use crate::persistence::banks::{BankData, BankOwner};
use shared::protocol::InventoryErrorReason;

const PRODUCT: u32 = 202310;

fn configure_unique_quest(app: &mut App, starter: bool) {
    let mut quests = app.world_mut().resource_mut::<QuestDb>();
    let quest = quests.quests.get_mut(&BEATING_THEM_BACK).unwrap();
    quest.objectives.clear();
    quest.choice_items.clear();
    quest.start_item = if starter { PRODUCT } else { 0 };
    quest.provided_item_count = 1;
    quest.reward_items = if starter {
        Vec::new()
    } else {
        vec![(PRODUCT, 1)]
    };
}

fn hold_product(app: &App, client: &Client, banked: bool) -> u64 {
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = crate::merchant::load_vendor_catalog(&conn).unwrap();
    crate::inventory::grant_item(
        db(app),
        app.world().resource::<ItemTemplates>(),
        &catalog,
        client.character_id,
        (PRODUCT, 1),
    )
    .unwrap();
    let mut record =
        persistence::inventory_store::load_inventory_record(db(app), client.character_id).unwrap();
    let product = record
        .inventory
        .items
        .iter()
        .find(|item| item.item_id == PRODUCT)
        .unwrap()
        .clone();
    let guid = product.item_guid;
    if banked {
        record.inventory.items.retain(|item| item.item_guid != guid);
        record
            .placements
            .retain(|placement| placement.item_guid != guid);
        let bank = BankData {
            tabs: vec![BankTabData {
                slots: vec![Some(product)],
                ..BankTabData::new("Unique reward".into())
            }],
            ..Default::default()
        };
        persistence::banks::save_bank_move(
            db(app),
            client.character_id,
            BankOwner::Character(client.character_id, &bank),
            Some(&record),
        );
    }
    guid
}

fn expect_cap_error(app: &mut App, client: &Client) {
    let errors = received::<InventoryError>(app, client);
    let failures = received::<QuestFailed>(app, client);
    assert_eq!(
        errors.len(),
        1,
        "quest failures={failures:?}; inventory={:?}",
        persistence::load_inventory(db(app), client.character_id)
    );
    assert_eq!(errors[0].reason, InventoryErrorReason::ItemMaxCount);
    assert!(
        failures.is_empty(),
        "ownership-cap refusal must not be mislabeled as a full inventory"
    );
}

#[test]
fn quest_reward_max_count_starter_refusal_keeps_closed_bank_product_and_log() {
    let mut app = setup_app().expect("authentic quest data");
    configure_unique_quest(&mut app, true);
    let client = spawn_client(&mut app, 1);
    let npc = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    let guid = hold_product(&app, &client, true);
    enter_world(&mut app, &client);
    let before =
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap();
    let log_before = load_quest_log(db(&app), client.character_id);
    let bank_before = persistence::banks::load_character_bank(db(&app), client.character_id);
    accept(&mut app, &client, npc, BEATING_THEM_BACK);
    expect_cap_error(&mut app, &client);
    assert_eq!(
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap(),
        before
    );
    assert_eq!(load_quest_log(db(&app), client.character_id), log_before);
    let bank = persistence::banks::load_character_bank(db(&app), client.character_id);
    assert_eq!(bank, bank_before);
    assert_eq!(bank.tabs[0].slots[0].as_ref().unwrap().item_guid, guid);
}

#[test]
fn quest_reward_max_count_turn_in_refusal_preserves_product_gold_and_unrewarded_log() {
    let mut app = setup_app().expect("authentic quest data");
    configure_unique_quest(&mut app, false);
    let client = spawn_client(&mut app, 1);
    let npc = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    let guid = hold_product(&app, &client, false);
    enter_world(&mut app, &client);
    accept(&mut app, &client, npc, BEATING_THEM_BACK);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let before =
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap();
    let log_before = load_quest_log(db(&app), client.character_id);
    turn_in(&mut app, &client, npc, BEATING_THEM_BACK, None);
    expect_cap_error(&mut app, &client);
    let after =
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap();
    assert_eq!(after, before);
    assert_eq!(load_quest_log(db(&app), client.character_id), log_before);
    assert_eq!(
        after
            .inventory
            .items
            .iter()
            .find(|item| item.item_id == PRODUCT)
            .unwrap()
            .item_guid,
        guid
    );
    assert!(received::<QuestGiverQuestComplete>(&mut app, &client).is_empty());
}
