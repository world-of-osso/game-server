//! Private native bank-reagent proof. The seed learns Tailoring (Bolt of Linen Cloth2963)
//! through the real trainer path in a test harness and copies those professions to the
//! private characters. Each character's two Linen Cloth sit only in its character bank or
//! only in its account's Warband bank; the native run performs one craft per character.
use super::*;
use crate::bank_storage::BankTabData;
use crate::persistence::banks::{
    BankData, BankOwner, load_account_bank, load_character_bank, save_bank_move,
};
use crate::persistence_accounts;
use std::path::{Path, PathBuf};

/// (account, character, linen held in the Warband bank rather than the character bank)
const OWNERS: [(&str, &str, bool); 4] = [
    ("fb_moderncraft_bankreag_cm", "Bkcharm", false),
    ("fb_moderncraft_bankreag_wm", "Bkwarbm", true),
    ("fb_moderncraft_bankreag_cf", "Bkcharf", false),
    ("fb_moderncraft_bankreag_wf", "Bkwarbf", true),
];

fn private_paths() -> (PathBuf, PathBuf) {
    let directory = Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-reagents-bank-2026-10-20/native/server/data",
    );
    (
        directory.join("bank-reagents.redb"),
        directory.join("world.db"),
    )
}

fn linen_tab(database: &redb::Database) -> BankData {
    let mut tab = BankTabData::new("Tab 1".into());
    tab.slots[0] = Some(InventoryItemData {
        item_guid: persistence::next_id(database, "item_instance"),
        item_id: LINEN_CLOTH,
        name: "Linen Cloth".into(),
        quality: 1,
        required_level: 0,
        stack_count: 2,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    BankData {
        tabs: vec![tab],
        ..Default::default()
    }
}

fn bank_linen(bank: &BankData) -> u32 {
    bank.tabs
        .iter()
        .flat_map(|tab| tab.slots.iter().flatten())
        .filter(|item| item.item_id == LINEN_CLOTH)
        .map(|item| item.stack_count)
        .sum()
}

fn bag_count_in(database: &redb::Database, id: u64, item_id: u32) -> u32 {
    persistence::load_inventory(database, id)
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| item.stack_count)
        .sum()
}

#[test]
#[ignore = "creates only the fixed private bank-reagent acceptance database"]
fn seed_private_bank_reagents_fixture() {
    let (path, _) = private_paths();
    assert!(!path.exists(), "never replace an acceptance database");
    let mut h = harness(10).unwrap();
    h.learn_tailoring();
    let professions = h.professions();
    assert!(
        professions
            .snapshot(h.app.world().resource::<ProfessionData>())
            .spells
            .contains(&BOLT_OF_LINEN_CLOTH)
    );
    let database = redb::Database::create(path).unwrap();
    persistence::create_tables(&database);
    for (account, name, warband) in OWNERS {
        let (account_id, _) =
            persistence_accounts::register_account(&database, account, "fbtest").unwrap();
        let (id, mut character) =
            persistence::create_character(&database, account_id, name, 1, 2).unwrap();
        character.level = 10;
        // The accepted cooldown seed's Stormwind spot.
        character.x = -8724.3;
        character.y = 475.3;
        character.z = 98.7;
        persistence::save_character(&database, id, &character);
        crate::maps::persistence::save_character_map(&database, id, 0);
        persistence::save_professions(&database, id, &professions);
        let bank = linen_tab(&database);
        let owner = if warband {
            BankOwner::Account(account_id, &bank)
        } else {
            BankOwner::Character(id, &bank)
        };
        save_bank_move(&database, id, owner, None);
        println!(
            "PRIVATE BANK REAGENTS account={account} account_id={account_id} character={name} id={id} warband={warband} bags_linen={} character_bank={} warband_bank={}",
            bag_count_in(&database, id, LINEN_CLOTH),
            bank_linen(&load_character_bank(&database, id)),
            bank_linen(&load_account_bank(&database, account_id)),
        );
    }
}

#[test]
#[ignore = "requires the native crafts and a stopped server"]
fn assert_private_bank_reagents_consumed() {
    let (path, _) = private_paths();
    let db = redb::Database::open(path).unwrap();
    for (account, name, warband) in OWNERS {
        let id = persistence::find_character_id_by_name(&db, name).unwrap();
        let account_id = persistence_accounts::list_accounts(&db)
            .into_iter()
            .find(|summary| summary.username == account)
            .unwrap()
            .account_id;
        let state = (
            bag_count_in(&db, id, BOLT_ITEM),
            bag_count_in(&db, id, LINEN_CLOTH),
            bank_linen(&load_character_bank(&db, id)),
            bank_linen(&load_account_bank(&db, account_id)),
        );
        println!(
            "PRIVATE BANK REAGENTS CONSUMED {name} warband={warband} (bolts, bag_linen, character_bank, warband_bank)={state:?}"
        );
        assert_eq!(
            state,
            (1, 0, 0, 0),
            "{name}: one bolt from its only bank source"
        );
    }
}
