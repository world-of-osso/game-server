//! Mailboxes: every mail a character has been sent, delivered or still in transit.
//!
//! Items travel inside the mail as `InventoryItemData` with their guid, so the
//! `item_instances` row keyed by that guid follows them into the recipient's bags.
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};

use super::InventoryItemData;
#[path = "persistence_mail_sender_migration.rs"]
mod sender_migration;
pub(crate) use sender_migration::migrate_sender_mailboxes;

/// Mailbox table: character_id -> bitcode `Vec<MailData>`.
pub(crate) const MAILBOXES: TableDefinition<u64, &[u8]> = TableDefinition::new("character_mail_v2");
/// Auction-only mailboxes (`Vec<LegacyAuctionMail>`, `u32` money); moved into
/// `MAILBOXES` as Auction House mail and dropped at startup.
pub(crate) const LEGACY_MAILBOXES: TableDefinition<u64, &[u8]> = TableDefinition::new("mailboxes");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub enum MailSender {
    Player { character_id: u64, name: String },
    AuctionHouse,
    CraftingOrder,
}

impl MailSender {
    pub fn name(&self) -> &str {
        match self {
            Self::Player { name, .. } => name,
            Self::AuctionHouse => "Auction House",
            Self::CraftingOrder => "Crafting Order",
        }
    }
}

/// An attached item at its fixed attachment slot.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct MailItemData {
    pub slot: u8,
    pub item: InventoryItemData,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct MailData {
    pub mail_id: u64,
    pub sender: MailSender,
    pub subject: String,
    pub body: String,
    pub items: Vec<MailItemData>,
    /// Copper attached.
    pub money: u64,
    /// Copper the recipient pays before taking the items.
    pub cod: u64,
    /// Unix seconds the mail reaches the inbox.
    pub delivers_at: u64,
    /// Unix seconds the mail is returned or deleted.
    pub expires_at: u64,
    pub read: bool,
    /// Sent back to its sender (AzerothCore `MAIL_CHECK_MASK_RETURNED`).
    pub returned: bool,
    /// Carries a C.O.D. payment (AzerothCore `MAIL_CHECK_MASK_COD_PAYMENT`).
    pub cod_payment: bool,
}

impl MailData {
    pub fn is_delivered(&self, now: u64) -> bool {
        self.delivers_at <= now
    }

    /// Player mail not yet sent back can be returned (AzerothCore
    /// `ObjectMgr::ReturnOrDeleteOldMails`: not from a creature, not returned, not
    /// a C.O.D. payment).
    pub fn returnable(&self) -> bool {
        matches!(self.sender, MailSender::Player { .. }) && !self.returned && !self.cod_payment
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.money == 0
    }
}

/// The auction mail layout before player mail existed.
#[derive(bitcode::Encode, bitcode::Decode)]
pub(crate) struct LegacyAuctionMail {
    pub mail_id: u64,
    pub subject: String,
    pub body: String,
    pub attached_money: u32,
    pub attached_item: Option<InventoryItemData>,
}

pub fn load_mailbox(db: &Database, character_id: u64) -> Vec<MailData> {
    let txn = db.begin_read().expect("Failed to begin mailbox read");
    let table = txn.open_table(MAILBOXES).expect("open mailboxes");
    read_mailbox(&table, character_id)
}

fn read_mailbox(
    table: &impl ReadableTable<u64, &'static [u8]>,
    character_id: u64,
) -> Vec<MailData> {
    table
        .get(character_id)
        .expect("read mailbox")
        .map(|row| bitcode::decode(row.value()).expect("decode mailbox"))
        .unwrap_or_default()
}

/// Replace a character's mailbox inside `txn`.
pub(crate) fn write_mailbox(txn: &WriteTransaction, character_id: u64, mails: &[MailData]) {
    txn.open_table(MAILBOXES)
        .expect("open mailboxes")
        .insert(character_id, bitcode::encode(mails).as_slice())
        .expect("write mailbox");
}

/// Add `mail` to a character's mailbox inside `txn`.
pub(crate) fn push_mail(txn: &WriteTransaction, character_id: u64, mail: MailData) {
    let mut table = txn.open_table(MAILBOXES).expect("open mailboxes");
    let mut mails = read_mailbox(&table, character_id);
    mails.push(mail);
    table
        .insert(character_id, bitcode::encode(&mails).as_slice())
        .expect("write mailbox");
}

/// Fixture delivery uses the runtime market transaction path.
#[cfg(test)]
pub fn deliver_mail(db: &Database, character_id: u64, mail: MailData) {
    let changes = crate::economy::store::MarketChanges {
        mails: vec![(character_id, mail)],
        ..Default::default()
    };
    crate::economy::store::commit_market(db, None, &changes).expect("deliver mail fixture");
}

/// Every mailbox (for the expiry sweep).
pub(crate) fn all_mailboxes(db: &Database) -> Vec<(u64, Vec<MailData>)> {
    let txn = db.begin_read().expect("Failed to begin mailbox read");
    let table = txn.open_table(MAILBOXES).expect("open mailboxes");
    table
        .iter()
        .expect("iterate mailboxes")
        .map(|row| {
            let (key, value) = row.expect("read mailbox row");
            let mails = bitcode::decode(value.value()).expect("decode mailbox");
            (key.value(), mails)
        })
        .collect()
}

/// Move every legacy auction mail into `MAILBOXES` as Auction House mail delivered
/// at `now`, and drop the old table, in one transaction.
pub(crate) fn migrate_legacy_mailboxes(db: &Database, now: u64) {
    let txn = db
        .begin_write()
        .expect("Failed to begin mailbox migration write");
    {
        let legacy = txn
            .open_table(LEGACY_MAILBOXES)
            .expect("Failed to open legacy mailboxes");
        let rows: Vec<(u64, Vec<LegacyAuctionMail>)> = legacy
            .iter()
            .expect("Failed to iterate legacy mailboxes")
            .map(|row| {
                let (key, value) = row.expect("Failed to read legacy mailbox row");
                let mails = bitcode::decode(value.value()).unwrap_or_else(|error| {
                    panic!("Failed to migrate mailbox {}: {error:?}", key.value())
                });
                (key.value(), mails)
            })
            .collect();
        for (character_id, mails) in rows {
            for mail in mails {
                push_mail(&txn, character_id, auction_mail_from_legacy(mail, now));
            }
        }
    }
    txn.delete_table(LEGACY_MAILBOXES)
        .expect("Failed to drop legacy mailboxes");
    crate::persistence_commit::commit(txn).expect("Failed to commit mailbox migration");
}

fn auction_mail_from_legacy(mail: LegacyAuctionMail, now: u64) -> MailData {
    MailData {
        mail_id: mail.mail_id,
        sender: MailSender::AuctionHouse,
        subject: mail.subject,
        body: mail.body,
        items: mail
            .attached_item
            .into_iter()
            .map(|item| MailItemData { slot: 0, item })
            .collect(),
        money: u64::from(mail.attached_money),
        cod: 0,
        delivers_at: now,
        expires_at: now + shared::mail::MAIL_EXPIRY_SECS,
        read: false,
        returned: false,
        cod_payment: false,
    }
}
