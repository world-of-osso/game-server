use super::*;
use crate::gold::GoldUpdateQueue;
use crate::persistence;

/// Take a mail's money and items at a mailbox (`TakeInboxMoney`, `TakeInboxItem`).
fn take_mail(db: &redb::Database, character_id: u64, mail_id: u64, gold: &mut GoldUpdateQueue) {
    use shared::protocol::MailAction;
    let templates = crate::item_templates::ItemTemplates::default();
    let mail = mail_store::load_mailbox(db, character_id)
        .into_iter()
        .find(|mail| mail.mail_id == mail_id)
        .expect("mail in the mailbox");
    let actions = std::iter::once(MailAction::TakeMoney).chain(mail.items.iter().map(|attached| {
        MailAction::TakeAttachment {
            slot: attached.slot,
        }
    }));
    for action in actions {
        let changes =
            crate::mail::act_on_mail(db, &templates, character_id, mail_id, action, unix_now())
                .expect("take from mail");
        for (id, copper) in changes.gold {
            gold.record(id, copper);
        }
    }
}

fn test_auction(id: u64, name: &str, quality: u8, required_level: u16) -> AuctionData {
    AuctionData {
        auction_id: id,
        seller_character_id: 1,
        seller_name: "Seller".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id: id as u32,
            name: name.into(),
            quality,
            required_level,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: 1,
        min_bid: 100,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(200),
        expires_at_unix_secs: unix_now() + 3600,
        faction: 0,
        deposit: 0,
    }
}

fn text_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        item_id: None,
        class_id: None,
        text: text.into(),
        page: 0,
        page_size: 50,
        min_level: None,
        max_level: None,
        quality: None,
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        faction: 0,
    }
}

#[path = "auction_house_tests/core.rs"]
mod core;
#[path = "auction_house_tests/lifecycle.rs"]
mod lifecycle;
#[path = "auction_house_tests/search.rs"]
mod search;
#[path = "auction_house_tests/settlement.rs"]
mod settlement;
