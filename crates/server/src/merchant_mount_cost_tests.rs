//! Retail TDB1210 vendors and local build 69933 ItemExtendedCost fixtures.
use super::*;
use shared::currency::{CurrencyEntry, CurrencyWallet};
use shared::protocol::{CurrencyStateUpdate, SelectGossipOption};
use shared::reputation::{CharacterReputation, Standing, threshold_for_standing};

fn shopper() -> Harness {
    let mut h = harness().expect("authentic mount vendor data");
    h.app.world_mut().entity_mut(h.player).insert((
        Level(80),
        CharacterReputation::default(),
        crate::currency::PlayerCurrencies(CurrencyWallet::default()),
    ));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert((pair::<SelectGossipOption>(), pair::<CurrencyStateUpdate>()));
    add_inbox::<CurrencyStateUpdate>(&mut h.app);
    h
}

fn offer(h: &mut Harness, entry: u32, item: u32) -> (Entity, VendorItem) {
    let npc = h.open(entry);
    let opening = h
        .take::<InteractionOpened>()
        .pop()
        .expect("vendor interaction");
    if let InteractionKind::Gossip(menu) = opening.kind {
        let option_id = menu.options.iter().find(|o| o.icon == 1).unwrap().option_id;
        h.send::<_, InteractionChannel>(SelectGossipOption {
            npc: npc.to_bits(),
            option_id,
        });
    }
    let list = h.take::<VendorInventory>().pop().expect("vendor list");
    let item = list
        .items
        .into_iter()
        .find(|o| o.item_id == item)
        .expect("authored mount offer");
    (npc, item)
}

fn wallet(h: &mut Harness, id: u32, amount: u32) {
    let wallet = CurrencyWallet {
        entries: vec![CurrencyEntry {
            id,
            amount,
            week_earned: 17,
        }],
    };
    persistence::save_currencies(
        &h.app.world().resource::<GameDatabase>().db,
        h.character_id,
        &wallet,
    );
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(crate::currency::PlayerCurrencies(wallet));
}

fn balance(h: &Harness, id: u32) -> u32 {
    h.app
        .world()
        .get::<crate::currency::PlayerCurrencies>(h.player)
        .unwrap()
        .0
        .get(id)
}

#[test]
fn currency_purchase_preserves_other_fifo_currency_changes_in_both_orders() {
    use bevy::ecs::system::RunSystemOnce;
    use std::time::{Duration, Instant};
    for currency_first in [true, false] {
        let mut h = shopper();
        wallet(&mut h, 395, 2175);
        let mut initial = h
            .app
            .world()
            .get::<crate::currency::PlayerCurrencies>(h.player)
            .unwrap()
            .0
            .clone();
        initial.entries.push(CurrencyEntry {
            id: 3,
            amount: 5,
            week_earned: 5,
        });
        persistence::save_currencies(
            &h.app.world().resource::<GameDatabase>().db,
            h.character_id,
            &initial,
        );
        h.app
            .world_mut()
            .get_mut::<crate::currency::PlayerCurrencies>(h.player)
            .unwrap()
            .0 = initial;
        let (npc, item) = offer(&mut h, 32216, 44230);
        let character = h.character_id;
        let enqueue_currency = |h: &Harness| {
            h.app.world().resource::<GameDatabase>().writer.enqueue(
                "adjacent currency actor",
                move |db| {
                    let mut wallet = persistence::load_currencies(db, character);
                    crate::currency::earn_currency(&mut wallet, 3, 17)?;
                    persistence::save_currencies(db, character, &wallet);
                    Ok(())
                },
            )
        };
        if currency_first {
            enqueue_currency(&h);
        }
        h.app
            .world_mut()
            .resource_mut::<PendingMerchant>()
            .requests
            .push_back(crate::merchant::actor::Request {
                connection: h.connection,
                character,
                player: h.player,
                vendor: npc,
                operation: crate::merchant::transactions::Operation::Buy(BuyItem {
                    npc: npc.to_bits(),
                    slot: item.slot,
                    item_id: item.item_id,
                    count: 1,
                    destination: None,
                }),
            });
        h.app
            .world_mut()
            .run_system_once(crate::merchant::actor::drive_merchant_commits)
            .unwrap();
        if !currency_first {
            enqueue_currency(&h);
        }
        let fence = h
            .app
            .world()
            .resource::<GameDatabase>()
            .writer
            .enqueue("currency purchase fixture fence", |_| Ok(()));
        let deadline = Instant::now() + Duration::from_secs(3);
        while fence.result().is_none() {
            assert!(Instant::now() < deadline, "currency purchase fence timeout");
            std::thread::sleep(Duration::from_millis(1));
        }
        fence.result().unwrap().unwrap();
        let durable =
            persistence::load_currencies(&h.app.world().resource::<GameDatabase>().db, character);
        assert_eq!(
            durable.get(3),
            22,
            "earlier currency actor lost; first={currency_first}"
        );
        assert_eq!(durable.get(395), 0);
        h.app
            .world_mut()
            .get_mut::<crate::currency::PlayerCurrencies>(h.player)
            .unwrap()
            .0 = durable;
        h.app
            .world_mut()
            .run_system_once(crate::merchant::actor::drive_merchant_commits)
            .unwrap();
        assert_eq!(
            balance(&h, 3),
            22,
            "later currency publication lost; first={currency_first}"
        );
        assert_eq!(h.count_of(44230), vec![1]);
    }
}

#[test]
fn mount_costs_mark_of_honor_real_vendor_multiplies_and_delivers() {
    let mut h = shopper();
    h.grant(137642, 31);
    let (npc, item) = offer(&mut h, 12783, 29468);
    assert_eq!(item.price, 0);
    assert_eq!(item.extra_costs.len(), 1);
    let cost = &item.extra_costs[0];
    assert_eq!(
        (cost.currency, cost.id, cost.amount, cost.icon_file_data_id),
        (false, 137642, 15, 1322720)
    );
    assert_eq!(cost.name, "Mark of Honor");
    h.buy(npc, item.slot, item.item_id, 2);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(137642), vec![1]);
    assert_eq!(h.count_of(29468), vec![1, 1]);
    assert_eq!(h.inventory().gold, 100_000);
    assert!(
        h.app
            .world()
            .resource::<GoldUpdateQueue>()
            .changed_inventories()
            .contains(&h.character_id)
    );
}

#[test]
fn mount_costs_insufficient_turnins_and_full_bags_preserve_payment() {
    let mut h = shopper();
    h.grant(137642, 14);
    let (npc, item) = offer(&mut h, 12783, 29468);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::MissingTurnins]);
    assert_eq!(h.inventory(), before);
    h.grant(137642, 17);
    h.grant(LARGE_ROUND_SHIELD, 12);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 2);
    assert_eq!(h.failures(), vec![MerchantError::InventoryFull]);
    assert_eq!(h.inventory(), before);
}

#[test]
fn mount_costs_currency_and_gold_refusals_are_atomic() {
    let mut h = shopper();
    wallet(&mut h, 241, 9);
    h.set_gold(10_000_000);
    h.app
        .world_mut()
        .get_mut::<CharacterReputation>(h.player)
        .unwrap()
        .set_rep(72, threshold_for_standing(Standing::Exalted));
    let (npc, item) = offer(&mut h, 33307, 46758);
    assert_eq!(item.price, 5_000_000);
    assert_eq!(
        (
            item.extra_costs[0].currency,
            item.extra_costs[0].id,
            item.extra_costs[0].amount
        ),
        (true, 241, 5)
    );
    assert_eq!(item.extra_costs[0].name, "Champion's Seal");
    assert_eq!(item.extra_costs[0].icon_file_data_id, 236246);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 2);
    assert_eq!(h.failures(), vec![MerchantError::MissingTurnins]);
    assert_eq!(h.inventory(), before);
    assert_eq!(balance(&h, 241), 9);
    wallet(&mut h, 241, 10);
    h.set_gold(9_999_999);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 2);
    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    assert_eq!(h.inventory(), before);
    assert_eq!(balance(&h, 241), 10);
    h.set_gold(10_000_000);
    h.grant(LARGE_ROUND_SHIELD, 13);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 2);
    assert_eq!(h.failures(), vec![MerchantError::InventoryFull]);
    assert_eq!(h.inventory(), before);
    assert_eq!(balance(&h, 241), 10);
    assert_eq!(
        persistence::load_currencies(&h.app.world().resource::<GameDatabase>().db, h.character_id)
            .get(241),
        10
    );
}

#[test]
fn mount_costs_currency_payment_and_inventory_survive_database_reopen() {
    let mut h = shopper();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("merchant.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (id, _) = persistence::create_character_with_test_bags(&db, 1, "Shopper", 1, 1).unwrap();
    assert_eq!(id, h.character_id);
    h.app.insert_resource(GameDatabase::new(db));
    wallet(&mut h, 241, 13);
    h.set_gold(10_000_007);
    h.app
        .world_mut()
        .get_mut::<CharacterReputation>(h.player)
        .unwrap()
        .set_rep(72, threshold_for_standing(Standing::Exalted));
    let (npc, item) = offer(&mut h, 33307, 46758);
    h.buy(npc, item.slot, item.item_id, 2);
    assert!(h.failures().is_empty());
    assert_eq!(balance(&h, 241), 3);
    let update = h
        .take::<CurrencyStateUpdate>()
        .pop()
        .expect("authoritative currency receipt");
    assert_eq!(
        update
            .snapshot
            .unwrap()
            .entries
            .iter()
            .find(|e| e.id == 241)
            .unwrap()
            .amount,
        3
    );
    assert_eq!(h.inventory().gold, 7);
    assert_eq!(h.count_of(46758), vec![1, 1]);
    drop(h);
    let db = redb::Database::open(path).unwrap();
    let inventory = persistence::load_inventory(&db, id);
    assert_eq!(inventory.gold, 7);
    assert_eq!(
        inventory
            .items
            .iter()
            .filter(|i| i.item_id == 46758)
            .map(|i| i.stack_count)
            .sum::<u32>(),
        2
    );
    let wallet = persistence::load_currencies(&db, id);
    assert_eq!(wallet.get(241), 3);
    assert_eq!(wallet.week_earned(241), 17);
}

#[test]
fn mount_costs_reputation_listing_and_purchase_use_actual_standing() {
    let mut h = shopper();
    h.set_gold(1_000_000);
    let (npc, item) = offer(&mut h, 20240, 29227);
    assert!(!item.usable);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::ReputationRequired]);
    assert_eq!(h.inventory(), before);
    h.app
        .world_mut()
        .get_mut::<CharacterReputation>(h.player)
        .unwrap()
        .set_rep(978, threshold_for_standing(Standing::Exalted) - 1);
    h.buy(npc, item.slot, item.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::ReputationRequired]);
    assert_eq!(h.inventory(), before);
    h.app
        .world_mut()
        .get_mut::<CharacterReputation>(h.player)
        .unwrap()
        .set_rep(978, threshold_for_standing(Standing::Exalted));
    let (npc, item) = offer(&mut h, 20240, 29227);
    assert!(item.usable);
    h.buy(npc, item.slot, item.item_id, 1);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(29227), vec![1]);
}

#[test]
fn mount_costs_two_item_price_rolls_back_first_turnin_when_second_is_missing() {
    let mut h = shopper();
    h.grant(26045, 140);
    h.grant(26044, 29);
    let (npc, item) = offer(&mut h, 21485, 28915);
    assert_eq!(
        item.extra_costs
            .iter()
            .map(|cost| (cost.currency, cost.id, cost.amount))
            .collect::<Vec<_>>(),
        vec![(false, 26045, 70), (false, 26044, 15)]
    );
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 2);
    assert_eq!(h.failures(), vec![MerchantError::MissingTurnins]);
    assert_eq!(h.inventory(), before);
    h.grant(26044, 1);
    h.buy(npc, item.slot, item.item_id, 2);
    assert!(h.failures().is_empty());
    assert!(h.count_of(26045).is_empty());
    assert!(h.count_of(26044).is_empty());
    assert_eq!(h.count_of(28915), vec![1, 1]);
}

#[test]
fn mount_costs_unavailable_metadata_is_not_a_free_offer_or_currency_alias() {
    let mut h = shopper();
    assert!(
        !h.app
            .world()
            .resource::<VendorCatalog>()
            .items(13216)
            .iter()
            .any(|slot| slot.item_id == 19030)
    );
    wallet(&mut h, 5, 1000); // Legacy fake Champion's Seal is not CurrencyTypes241.
    let (npc, item) = offer(&mut h, 33307, 45125);
    let before = h.inventory();
    h.buy(npc, item.slot, item.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::MissingTurnins]);
    assert_eq!(h.inventory(), before);
    assert_eq!(balance(&h, 5), 1000);
}

#[test]
fn mount_costs_currency_precision_and_faction_filter_remain_authoritative() {
    let mut h = shopper();
    wallet(&mut h, 395, 2175);
    let (npc, item) = offer(&mut h, 32216, 44230);
    assert_eq!(item.extra_costs[0].id, 395);
    assert_eq!(item.extra_costs[0].amount, 2175);
    h.buy(npc, item.slot, item.item_id, 1);
    assert!(h.failures().is_empty());
    assert_eq!(balance(&h, 395), 0);
    let horde_slot = h
        .app
        .world()
        .resource::<VendorCatalog>()
        .items(32216)
        .iter()
        .position(|s| s.item_id == 44231)
        .unwrap() as u32;
    h.buy(npc, horde_slot, 44231, 1);
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    assert!(h.count_of(44231).is_empty());
}
