//! Bag slots (TrinityCore `Player::SwapItem`, `CanUnequipItem`, `ItemCanGoIntoBag`)
//! with Small Brown Pouch (6 slots), Blue Leather Bag (8) and Simply Stitched
//! Reagent Bag (32).
use shared::protocol::InventoryErrorReason as Reason;

use super::*;
use crate::persistence::inventory_store::EquippedBagData;

fn cell(bag: u8, slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag, slot }
}

fn slot(slot: EquipmentSlot) -> ItemLocation {
    ItemLocation::Equipment(slot)
}

/// A character in the world with linen, peacebloom and ore in backpack slots 0-2 and
/// `items` after them (slot 3 on); the enter-world snapshots are drained.
fn bagger(app: &mut App, items: &[(u32, &str)]) -> (Client, Vec<u64>) {
    let client = spawn_client(app, 1, "Bagger");
    let guids = items
        .iter()
        .map(|&(item_id, name)| grant_bag_item(app, client.character_id, item_id, name))
        .collect();
    enter_world(app, &client);
    received::<InventorySnapshot>(app, &client);
    received::<EquipmentSnapshot>(app, &client);
    (client, guids)
}

fn swap(app: &mut App, client: &Client, from: ItemLocation, to: ItemLocation) {
    request(app, client, SwapItem { from, to });
}

fn errors(app: &mut App, client: &Client) -> Vec<Reason> {
    received::<InventoryError>(app, client)
        .into_iter()
        .map(|error| error.reason)
        .collect()
}

fn sizes(snapshot: &InventorySnapshot) -> Vec<(u8, u8)> {
    snapshot
        .bags
        .iter()
        .map(|bag| (bag.bag, bag.size))
        .collect()
}

fn reload(app: &App, client: &Client) -> InventoryView {
    let templates = app.world().resource::<ItemTemplates>();
    load_model(db(app), templates, client.character_id)
        .unwrap()
        .view()
}

#[test]
fn equipping_a_pouch_from_the_backpack_opens_a_six_slot_bag_one() {
    let mut app = setup_app(persistence::temp_database());
    let (client, guids) = bagger(&mut app, &[(SMALL_BROWN_POUCH, "Small Brown Pouch")]);
    let linen = guid_of(&app, client.character_id, LINEN);

    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));

    assert_eq!(errors(&mut app, &client), vec![]);
    let snapshots = received::<InventorySnapshot>(&mut app, &client);
    assert_eq!(snapshots.len(), 1, "a bag slot change resends the sizes");
    assert_eq!(
        sizes(&snapshots[0]),
        vec![(0, 16), (1, 6), (2, 0), (3, 0), (4, 0), (5, 0)]
    );
    assert_eq!(
        received::<InventoryDelta>(&mut app, &client)[0].changes,
        vec![
            InventorySlotChange {
                location: bag(3),
                item: None,
            },
            InventorySlotChange {
                location: slot(EquipmentSlot::Bag1),
                item: Some(stack(guids[0], SMALL_BROWN_POUCH, 1)),
            },
        ]
    );

    swap(&mut app, &client, bag(0), cell(1, 5));

    assert_eq!(errors(&mut app, &client), vec![]);
    assert!(received::<InventorySnapshot>(&mut app, &client).is_empty());
    let view = reload(&app, &client);
    assert_eq!(view.get(&cell(1, 5)), Some(&stack(linen, LINEN, 20)));
    assert_eq!(view.get(&bag(0)), None);
    swap(&mut app, &client, bag(1), cell(1, 6));
    assert_eq!(
        errors(&mut app, &client),
        vec![Reason::InternalBagError],
        "bag 1 has six slots"
    );
}

#[test]
fn an_equipped_bag_is_unequipped_only_when_empty_and_never_into_itself() {
    let mut app = setup_app(persistence::temp_database());
    let (client, _) = bagger(&mut app, &[(SMALL_BROWN_POUCH, "Small Brown Pouch")]);
    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));
    swap(&mut app, &client, bag(0), cell(1, 0));
    received::<InventorySnapshot>(&mut app, &client);
    errors(&mut app, &client);

    swap(&mut app, &client, slot(EquipmentSlot::Bag1), bag(5));
    assert_eq!(errors(&mut app, &client), vec![Reason::DestroyNonemptyBag]);
    swap(&mut app, &client, slot(EquipmentSlot::Bag1), cell(1, 1));
    assert_eq!(errors(&mut app, &client), vec![Reason::BagInBag]);
    let record = load_inventory_record(db(&app), client.character_id).unwrap();
    assert_eq!(record.bag_slots.len(), 1, "the pouch stays equipped");

    swap(&mut app, &client, cell(1, 0), bag(0));
    swap(&mut app, &client, slot(EquipmentSlot::Bag1), bag(5));

    assert_eq!(errors(&mut app, &client), vec![]);
    let snapshots = received::<InventorySnapshot>(&mut app, &client);
    assert_eq!(sizes(snapshots.last().unwrap())[1], (1, 0));
    let view = reload(&app, &client);
    assert_eq!(
        view.get(&bag(5)).map(|item| item.item_id),
        Some(SMALL_BROWN_POUCH)
    );
    assert_eq!(view.get(&slot(EquipmentSlot::Bag1)), None);
}

#[test]
fn swapping_bags_moves_the_items_into_the_new_bag_only_when_they_fit() {
    let mut app = setup_app(persistence::temp_database());
    let swords = [(WORN_SHORTSWORD, "Worn Shortsword"); 7];
    let items: Vec<_> = [
        (BLUE_LEATHER_BAG, "Blue Leather Bag"),
        (SMALL_BROWN_POUCH, "Small Brown Pouch"),
    ]
    .into_iter()
    .chain(swords)
    .collect();
    let (client, guids) = bagger(&mut app, &items);
    let (blue, pouch) = (guids[0], guids[1]);
    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));
    for (index, sword_slot) in (5..12).enumerate() {
        swap(&mut app, &client, bag(sword_slot), cell(1, index as u8));
    }
    assert_eq!(errors(&mut app, &client), vec![]);

    // Seven swords do not fit the 6-slot pouch.
    swap(&mut app, &client, bag(4), slot(EquipmentSlot::Bag1));
    assert_eq!(errors(&mut app, &client), vec![Reason::CantSwap]);
    assert_eq!(
        reload(&app, &client).get(&slot(EquipmentSlot::Bag1)),
        Some(&stack(blue, BLUE_LEATHER_BAG, 1))
    );

    // With six they move into the pouch, packed from its first slot.
    swap(&mut app, &client, cell(1, 6), bag(5));
    received::<InventorySnapshot>(&mut app, &client);
    swap(&mut app, &client, bag(4), slot(EquipmentSlot::Bag1));

    assert_eq!(errors(&mut app, &client), vec![]);
    let view = reload(&app, &client);
    assert_eq!(
        view.get(&slot(EquipmentSlot::Bag1)),
        Some(&stack(pouch, SMALL_BROWN_POUCH, 1))
    );
    assert_eq!(view.get(&bag(4)), Some(&stack(blue, BLUE_LEATHER_BAG, 1)));
    let in_pouch: Vec<_> = (0..6)
        .map(|index| view.get(&cell(1, index)).map(|item| item.item_id))
        .collect();
    assert_eq!(in_pouch, vec![Some(WORN_SHORTSWORD); 6]);
    let snapshots = received::<InventorySnapshot>(&mut app, &client);
    assert_eq!(sizes(&snapshots[0])[1], (1, 6));
    assert_eq!(snapshots[0].bags[1].items.len(), 6);
}

#[test]
fn a_bag_cannot_replace_the_bag_it_sits_in() {
    let mut app = setup_app(persistence::temp_database());
    let pouch = (SMALL_BROWN_POUCH, "Small Brown Pouch");
    let (client, _) = bagger(&mut app, &[pouch, pouch]);
    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));
    swap(&mut app, &client, bag(4), cell(1, 0));
    assert_eq!(errors(&mut app, &client), vec![]);

    swap(&mut app, &client, cell(1, 0), slot(EquipmentSlot::Bag1));

    assert_eq!(errors(&mut app, &client), vec![Reason::CantSwap]);
}

#[test]
fn only_bags_go_in_bag_slots_and_the_reagent_bag_holds_only_reagents() {
    let mut app = setup_app(persistence::temp_database());
    let (client, _) = bagger(
        &mut app,
        &[
            (REAGENT_BAG, "Simply Stitched Reagent Bag"),
            (WORN_SHORTSWORD, "Worn Shortsword"),
        ],
    );

    swap(&mut app, &client, bag(4), slot(EquipmentSlot::Bag2));
    assert_eq!(errors(&mut app, &client), vec![Reason::WrongSlot]);
    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));
    assert_eq!(errors(&mut app, &client), vec![Reason::WrongSlot]);

    request(&mut app, &client, EquipItem { from: bag(3) });
    assert_eq!(errors(&mut app, &client), vec![]);
    let snapshots = received::<InventorySnapshot>(&mut app, &client);
    assert_eq!(sizes(&snapshots[0])[5], (5, 32));

    swap(&mut app, &client, bag(4), cell(5, 0));
    assert_eq!(errors(&mut app, &client), vec![Reason::WrongBagType]);
    swap(&mut app, &client, bag(0), cell(5, 0));
    assert_eq!(errors(&mut app, &client), vec![]);
    assert_eq!(
        reload(&app, &client)
            .get(&cell(5, 0))
            .map(|item| item.item_id),
        Some(LINEN)
    );
}

#[test]
fn equipped_bags_and_their_items_survive_a_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.redb");
    let db = Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let mut app = setup_app(db);
    let (client, guids) = bagger(&mut app, &[(SMALL_BROWN_POUCH, "Small Brown Pouch")]);
    let linen = guid_of(&app, client.character_id, LINEN);
    swap(&mut app, &client, bag(3), slot(EquipmentSlot::Bag1));
    swap(&mut app, &client, bag(0), cell(1, 4));
    assert_eq!(errors(&mut app, &client), vec![]);
    let character_id = client.character_id;
    drop(app);

    let db = Database::open(&path).unwrap();
    let record = load_inventory_record(&db, character_id).unwrap();
    assert_eq!(
        record
            .bag_slots
            .iter()
            .map(|EquippedBagData { bag, item }| (*bag, item.item_guid))
            .collect::<Vec<_>>(),
        vec![(1, guids[0])]
    );
    let model = load_model(&db, &test_templates(), character_id).unwrap();
    let snapshot = model.inventory_snapshot();
    assert_eq!(sizes(&snapshot)[1], (1, 6));
    assert_eq!(
        snapshot.bags[1].items,
        vec![BagSlotItem {
            slot: 4,
            item: stack(linen, LINEN, 20),
        }]
    );
}
