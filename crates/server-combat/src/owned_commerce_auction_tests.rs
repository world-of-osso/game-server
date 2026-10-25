use super::*;
use crate::owned_commerce_source_tests::{Catalog, assert_owned, assert_stacks, character};
use crate::persistence::inventory_store::load_item_definition_source;
use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
use shared::protocol::MailAction;

#[test]
fn owned_commerce_sources_auction_mints_and_settles_both_catalogs_through_mail_reopen() {
    let catalog = Catalog::load();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let seller = character(&db, "Seller");
    let buyer = character(&db, "Buyer");
    let guids = catalog.seed(&db, seller, &[(Retail, 2512, 7), (Forever70205, 2512, 7)]);
    catalog.seed(&db, buyer, &[]);
    let mut state = AuctionHouseState::default();
    let mut expected = Vec::new();
    for (&guid, source) in guids.iter().zip([Retail, Forever70205]) {
        let response = create_auction(
            &db,
            &catalog.templates,
            &mut state,
            seller,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: guid,
                stack_count: 3,
                min_bid: 100,
                buyout_price: Some(200),
                duration: AuctionDuration::Short,
            },
        );
        assert!(response.success, "{}", response.message);
        let listing = state
            .auctions
            .values()
            .find(|a| load_item_definition_source(&db, a.item.item_guid) == source)
            .unwrap();
        assert_ne!(listing.item.item_guid, guid);
        expected.push((listing.item.item_guid, 2512, source, 3));
    }
    assert_eq!(state.auctions.len(), 2);
    assert_owned(
        &db,
        &catalog,
        seller,
        &[
            (guids[0], 2512, Retail, 4),
            (guids[1], 2512, Forever70205, 4),
        ],
    );
    drop(state);
    drop(db);

    let db = redb::Database::open(file.path()).unwrap();
    let mut state = AuctionHouseState {
        initialized: true,
        auctions: persistence::load_auctions(&db)
            .into_iter()
            .map(|a| (a.auction_id, a))
            .collect(),
        ..Default::default()
    };
    assert_eq!(state.auctions.len(), 2);
    let ids: Vec<_> = state.auctions.keys().copied().collect();
    for id in ids {
        let response = buyout_auction(&db, &mut state, buyer, id);
        assert!(response.success, "{}", response.message);
    }
    assert!(state.auctions.is_empty());
    assert!(persistence::load_auctions(&db).is_empty());
    assert_eq!(persistence::load_inventory(&db, buyer).gold, 9_600);
    drop(db);

    let db = redb::Database::open(file.path()).unwrap();
    let contents = crate::mail::mailbox_contents(&db, &catalog.templates, buyer, 0);
    assert_eq!(contents.mails.len(), 2);
    assert_stacks(
        contents
            .mails
            .iter()
            .flat_map(|m| m.attachments.iter().map(|a| a.item.clone())),
        &expected,
    );
    for mail in contents.mails {
        crate::mail::act_on_mail(
            &db,
            &catalog.templates,
            buyer,
            mail.mail_id,
            MailAction::TakeAttachment { slot: 0 },
            unix_now(),
        )
        .unwrap();
    }
    assert!(
        mail_store::load_mailbox(&db, buyer)
            .iter()
            .all(|m| m.items.is_empty())
    );
    drop(db);

    let db = redb::Database::open(file.path()).unwrap();
    assert_owned(&db, &catalog, buyer, &expected);
    assert_owned(
        &db,
        &catalog,
        seller,
        &[
            (guids[0], 2512, Retail, 4),
            (guids[1], 2512, Forever70205, 4),
        ],
    );
    assert_eq!(persistence::load_inventory(&db, buyer).gold, 9_600);
    assert!(persistence::load_auctions(&db).is_empty());
}
