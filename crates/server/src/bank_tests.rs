//! Real-DB tests against data/world.db.
use shared::protocol::*;

use crate::bank_test_harness::{COPPER_ORE, Harness, LINEN, PEACEBLOOM, harness};

fn last_contents(h: &mut Harness, bank: BankType) -> BankContents {
    h.take::<BankContents>()
        .into_iter()
        .rev()
        .find(|contents| contents.bank == bank)
        .expect("bank contents sent")
}

fn failures(h: &mut Harness) -> Vec<BankError> {
    h.take::<BankFailed>()
        .into_iter()
        .map(|failed| failed.error)
        .collect()
}

fn slot_items(contents: &BankContents, tab: usize) -> Vec<(usize, u32, u32)> {
    contents.tabs[tab]
        .slots
        .iter()
        .enumerate()
        .filter_map(|(slot, item)| item.as_ref().map(|item| (slot, item.item_id, item.count)))
        .collect()
}

fn buy_tab(h: &mut Harness, npc: bevy::prelude::Entity, bank: BankType) {
    h.send::<_, BankChannel>(BankPurchaseTab {
        npc: npc.to_bits(),
        bank,
    });
}

/// Retail `ItemUtil.GetCraftingReagentCount` counts both banks away from a banker,
/// so a character entering the world learns their contents.
#[test]
fn entering_the_world_sends_both_banks_without_a_banker() {
    use crate::bank_storage::BankTabData;
    use crate::persistence::banks::{BankData, BankOwner, save_bank_move};
    let Some(mut h) = harness() else { return };
    let alt = h.create_alt("Alt");
    let linen = |count| {
        let mut tab = BankTabData::new("Tab 1".into());
        tab.slots[3] = Some(crate::persistence::InventoryItemData {
            item_guid: crate::persistence::next_id(h.db(), "item_instance"),
            item_id: LINEN,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 0,
            stack_count: count,
            vendor_sell_price: 0,
            unique_auction: false,
        });
        BankData {
            tabs: vec![tab],
            ..Default::default()
        }
    };
    let (character, warband) = (linen(2), linen(5));
    save_bank_move(h.db(), alt, BankOwner::Character(alt, &character), None);
    save_bank_move(
        h.db(),
        alt,
        BankOwner::Account(crate::bank_test_harness::ACCOUNT, &warband),
        None,
    );

    let connection = h.connect_character(alt, "Alt");
    h.app.update();
    let sent = h.take_for::<BankContents>(connection);
    let linen_of = |bank| {
        sent.iter()
            .rev()
            .find(|contents| contents.bank == bank)
            .map(|contents| slot_items(contents, 0))
    };
    assert_eq!(linen_of(BankType::Character), Some(vec![(3, LINEN, 2)]));
    assert_eq!(linen_of(BankType::Account), Some(vec![(3, LINEN, 5)]));
}

#[test]
fn opening_the_banker_sends_both_empty_banks_with_retail_first_tab_prices() {
    let Some(mut h) = harness() else { return };
    h.open_banker();
    let sent = h.take::<BankContents>();
    let character = sent.iter().find(|c| c.bank == BankType::Character).unwrap();
    let account = sent.iter().find(|c| c.bank == BankType::Account).unwrap();
    assert!(character.tabs.is_empty());
    // Retail BankTab: first character tab 1g, first Warband tab 1000g.
    assert_eq!(character.next_tab_cost, Some(10_000));
    assert_eq!(character.money, None);
    assert_eq!(account.next_tab_cost, Some(10_000_000));
    assert_eq!(account.money, Some(0));
}

#[test]
fn buying_a_character_tab_charges_the_player_and_prices_the_next() {
    let Some(mut h) = harness() else { return };
    let npc = h.open_banker();
    assert_eq!(h.gold(), 100_000);
    buy_tab(&mut h, npc, BankType::Character);
    let contents = last_contents(&mut h, BankType::Character);
    assert_eq!(h.gold(), 90_000);
    assert_eq!(contents.tabs.len(), 1);
    assert_eq!(contents.tabs[0].name, "Tab 1");
    assert_eq!(contents.tabs[0].slots.len(), BANK_TAB_SLOTS);
    assert_eq!(contents.next_tab_cost, Some(1_000_000));
    // 100g second tab with 9g left.
    buy_tab(&mut h, npc, BankType::Character);
    assert_eq!(failures(&mut h), vec![BankError::CantAfford]);
    assert_eq!(h.gold(), 90_000);
}

#[test]
fn linen_deposited_into_the_character_bank_comes_back_on_withdraw() {
    let Some(mut h) = harness() else { return };
    let npc = h.open_banker();
    buy_tab(&mut h, npc, BankType::Character);
    let linen = h.guid_of(LINEN);
    h.send::<_, BankChannel>(BankDeposit {
        npc: npc.to_bits(),
        bank: BankType::Character,
        tab: 0,
        item_guid: linen,
    });
    let contents = last_contents(&mut h, BankType::Character);
    assert_eq!(slot_items(&contents, 0), vec![(0, LINEN, 20)]);
    assert_eq!(h.count_of(LINEN), 0);

    h.send::<_, BankChannel>(BankWithdraw {
        npc: npc.to_bits(),
        bank: BankType::Character,
        tab: 0,
        slot: 0,
    });
    let contents = last_contents(&mut h, BankType::Character);
    assert!(slot_items(&contents, 0).is_empty());
    assert_eq!(h.count_of(LINEN), 20);
    assert!(failures(&mut h).is_empty());
}

#[test]
fn deposits_need_a_purchased_tab_and_an_open_banker() {
    let Some(mut h) = harness() else { return };
    let linen = h.guid_of(LINEN);
    // No banker open.
    h.send::<_, BankChannel>(BankDeposit {
        npc: 42,
        bank: BankType::Character,
        tab: 0,
        item_guid: linen,
    });
    assert_eq!(failures(&mut h), vec![BankError::NotInteracting]);
    let npc = h.open_banker();
    h.send::<_, BankChannel>(BankDeposit {
        npc: npc.to_bits(),
        bank: BankType::Character,
        tab: 0,
        item_guid: linen,
    });
    assert_eq!(failures(&mut h), vec![BankError::TabLocked]);
    assert_eq!(h.count_of(LINEN), 20);
}

#[test]
fn warband_bank_is_shared_by_characters_of_the_account() {
    let Some(mut h) = harness() else { return };
    let first = h.character_id;
    let alt = h.create_alt("Bankalt");
    h.set_gold(10_100_000);
    let npc = h.open_banker();
    buy_tab(&mut h, npc, BankType::Account);
    assert_eq!(h.gold(), 100_000);
    let ore = h.guid_of(COPPER_ORE);
    h.send::<_, BankChannel>(BankDeposit {
        npc: npc.to_bits(),
        bank: BankType::Account,
        tab: 0,
        item_guid: ore,
    });
    assert_eq!(h.count_of(COPPER_ORE), 0);

    h.play_as(alt);
    let npc = h.open_banker();
    let contents = last_contents(&mut h, BankType::Account);
    assert_eq!(slot_items(&contents, 0), vec![(0, COPPER_ORE, 10)]);
    h.send::<_, BankChannel>(BankWithdraw {
        npc: npc.to_bits(),
        bank: BankType::Account,
        tab: 0,
        slot: 0,
    });
    // The alt started with 10 Copper Ore of its own.
    assert_eq!(h.count_of(COPPER_ORE), 20);
    assert!(slot_items(&last_contents(&mut h, BankType::Account), 0).is_empty());
    h.play_as(first);
    assert_eq!(h.count_of(COPPER_ORE), 0);
}

#[test]
fn soulbound_items_stay_out_of_the_warband_bank() {
    let Some(mut h) = harness() else { return };
    h.set_gold(10_100_000);
    let npc = h.open_banker();
    buy_tab(&mut h, npc, BankType::Account);
    let linen = h.guid_of(LINEN);
    h.make_soulbound(linen);
    h.send::<_, BankChannel>(BankDeposit {
        npc: npc.to_bits(),
        bank: BankType::Account,
        tab: 0,
        item_guid: linen,
    });
    assert_eq!(failures(&mut h), vec![BankError::SoulboundInAccountBank]);
    assert_eq!(h.count_of(LINEN), 20);
}

#[test]
fn only_the_warband_bank_holds_money() {
    let Some(mut h) = harness() else { return };
    let npc = h.open_banker();
    let transfer = |copper, deposit, bank| BankMoneyTransfer {
        npc: npc.to_bits(),
        bank,
        copper,
        deposit,
    };
    h.send::<_, BankChannel>(transfer(5_000, true, BankType::Account));
    assert_eq!(last_contents(&mut h, BankType::Account).money, Some(5_000));
    assert_eq!(h.gold(), 95_000);
    h.send::<_, BankChannel>(transfer(6_000, false, BankType::Account));
    h.send::<_, BankChannel>(transfer(100, true, BankType::Character));
    assert_eq!(
        failures(&mut h),
        vec![BankError::NotEnoughMoney, BankError::NoMoneyTransfer]
    );
    h.send::<_, BankChannel>(transfer(2_000, false, BankType::Account));
    assert_eq!(last_contents(&mut h, BankType::Account).money, Some(3_000));
    assert_eq!(h.gold(), 97_000);
}

#[test]
fn deposit_all_reagents_moves_trade_goods_into_the_assigned_tab() {
    let Some(mut h) = harness() else { return };
    h.set_gold(2_000_000);
    let npc = h.open_banker();
    buy_tab(&mut h, npc, BankType::Character);
    buy_tab(&mut h, npc, BankType::Character);
    h.send::<_, BankChannel>(BankUpdateTabSettings {
        npc: npc.to_bits(),
        bank: BankType::Character,
        tab: 1,
        name: "Reagents".into(),
        icon: 132889,
        deposit_flags: bank_tab_flags::REAGENTS,
    });
    h.send::<_, BankChannel>(BankAutoDeposit {
        npc: npc.to_bits(),
        bank: BankType::Character,
        include_reagents: false,
    });
    let contents = last_contents(&mut h, BankType::Character);
    assert_eq!(contents.tabs[1].name, "Reagents");
    assert!(slot_items(&contents, 0).is_empty());
    assert_eq!(
        slot_items(&contents, 1),
        vec![(0, LINEN, 20), (1, PEACEBLOOM, 10), (2, COPPER_ORE, 10)]
    );
    assert_eq!(h.inventory().items.len(), 0);
}

#[test]
fn tab_names_longer_than_fifteen_letters_are_refused() {
    let Some(mut h) = harness() else { return };
    let npc = h.open_banker();
    buy_tab(&mut h, npc, BankType::Character);
    h.send::<_, BankChannel>(BankUpdateTabSettings {
        npc: npc.to_bits(),
        bank: BankType::Character,
        tab: 0,
        name: "Sixteen letters!".into(),
        icon: 0,
        deposit_flags: 0,
    });
    assert_eq!(failures(&mut h), vec![BankError::InvalidTabName]);
}

#[test]
fn all_five_warband_tabs_are_bought_with_gold_above_the_old_u32_cap() {
    let Some(mut h) = harness() else { return };
    // 1000g + 25,000g + 100,000g + 500,000g + 2,500,000g (Retail BankTab).
    h.set_gold(31_260_000_000 + 7);
    let npc = h.open_banker();
    for _ in 0..5 {
        buy_tab(&mut h, npc, BankType::Account);
    }
    let contents = last_contents(&mut h, BankType::Account);
    assert_eq!(contents.tabs.len(), 5);
    assert_eq!(contents.next_tab_cost, None);
    assert_eq!(h.gold(), 7);
    buy_tab(&mut h, npc, BankType::Account);
    assert_eq!(failures(&mut h), vec![BankError::MaxTabs]);
}

#[test]
fn warband_money_stops_at_the_retail_gold_cap() {
    let Some(mut h) = harness() else { return };
    h.set_gold(crate::gold::MAX_MONEY);
    let npc = h.open_banker();
    let transfer = |copper, deposit| BankMoneyTransfer {
        npc: npc.to_bits(),
        bank: BankType::Account,
        copper,
        deposit,
    };
    h.send::<_, BankChannel>(transfer(crate::gold::MAX_MONEY, true));
    assert_eq!(
        last_contents(&mut h, BankType::Account).money,
        Some(99_999_999_999)
    );
    assert_eq!(h.gold(), 0);
    h.set_gold(1);
    h.send::<_, BankChannel>(transfer(1, true));
    // Withdrawing it all back fits; one more copper on the player would not.
    h.send::<_, BankChannel>(transfer(crate::gold::MAX_MONEY, false));
    assert_eq!(
        failures(&mut h),
        vec![BankError::TooMuchGold, BankError::TooMuchGold]
    );
    assert_eq!(h.gold(), 1);
}

#[test]
fn achievements_criteria_bank_tab_purchases_set_real_counts_per_bank_type() {
    use crate::achievement::PlayerAchievementState;
    let Some(mut h) = harness() else { return };
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.init_resource::<crate::maps::ServerClock>();
    let player = h.player;
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    let progress = |h: &crate::bank_test_harness::Harness, criterion: u32| {
        crate::persistence::load_achievements(h.db(), h.character_id)
            .criteria_progress
            .get(&criterion)
            .copied()
    };
    let npc = h.open_banker();
    // Character criterion105270 (Safer Deposit) and Warband criterion67053.
    buy_tab(&mut h, npc, BankType::Character);
    assert!(failures(&mut h).is_empty());
    assert_eq!((progress(&h, 105270), progress(&h, 67053)), (Some(1), None));
    // A refused purchase changes no count.
    buy_tab(&mut h, npc, BankType::Character);
    assert_eq!(failures(&mut h), vec![BankError::CantAfford]);
    assert_eq!(progress(&h, 105270), Some(1));
    h.set_gold(10_000_000);
    buy_tab(&mut h, npc, BankType::Account);
    assert!(failures(&mut h).is_empty());
    assert_eq!(
        (progress(&h, 105270), progress(&h, 67053)),
        (Some(1), Some(1))
    );
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    // Learning to Share40142: the first Warband tab is an account achievement.
    assert!(state.account.as_ref().unwrap().earned_ids.contains(&40142));
    assert!(!state.earned_ids.contains(&42191));
}
