//! In-game mail system: send items, gold, and messages between players.
//!
//! Ref: AzerothCore `Mail.h`, `MailHandler.cpp`.

use serde::{Deserialize, Serialize};

/// An item attached to a mail message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAttachment {
    pub item_id: u32,
    pub count: u16,
}

/// A mail message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mail {
    /// Unique mail ID.
    pub id: u64,
    /// Sender character name (or "Auction House", "System", etc).
    pub sender: String,
    /// Recipient character name.
    pub recipient: String,
    /// Subject line.
    pub subject: String,
    /// Message body.
    pub body: String,
    /// Attached items.
    pub attachments: Vec<MailAttachment>,
    /// Gold attached in copper.
    pub gold: u32,
    /// Cash on delivery amount in copper (0 = no COD).
    pub cod: u32,
    /// Server timestamp when sent (seconds since epoch).
    pub sent_at: u64,
    /// Server timestamp when the mail becomes available to the recipient.
    pub delivers_at: u64,
    /// Server timestamp when the mail expires.
    pub expires_at: u64,
    /// Whether the mail has been read.
    pub read: bool,
    /// Whether attachments/gold have been collected.
    pub collected: bool,
}

/// Cost to send a mail in copper (base 30c per WoW standard).
const MAIL_SEND_COST: u32 = 30;
/// Additional cost per attachment.
const MAIL_ATTACHMENT_COST: u32 = 30;
/// Maximum attachments per mail.
pub const MAX_ATTACHMENTS: usize = 12;

/// Delivery delay for mails with items or gold (1 hour in seconds).
const ITEM_DELIVERY_DELAY: u64 = 3600;

/// Compute the delivery delay for a mail.
///
/// - Same account: instant (0 delay).
/// - Text only (no items, no gold): instant.
/// - Items or gold: 1 hour delay.
pub fn delivery_delay(has_attachments: bool, has_gold: bool, same_account: bool) -> u64 {
    if same_account {
        return 0;
    }
    if has_attachments || has_gold {
        ITEM_DELIVERY_DELAY
    } else {
        0
    }
}

/// Why sending a mail failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailError {
    /// Too many attachments.
    TooManyAttachments,
    /// Recipient not found.
    RecipientNotFound,
    /// Can't mail to yourself.
    SelfMail,
    /// Not enough gold to cover send cost + attached gold.
    NotEnoughGold,
}

/// Calculate the cost to send a mail (in copper).
pub fn send_cost(attachment_count: usize) -> u32 {
    MAIL_SEND_COST + attachment_count as u32 * MAIL_ATTACHMENT_COST
}

/// Parameters for creating a new mail.
pub struct NewMail {
    pub id: u64,
    pub sender: String,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<MailAttachment>,
    pub gold: u32,
    pub cod: u32,
    pub now: u64,
    pub delivery_delay: u64,
    pub expiry_days: u32,
}

/// Validate and create a mail. Returns the mail or an error.
pub fn create_mail(params: NewMail) -> Result<Mail, MailError> {
    if params.sender == params.recipient {
        return Err(MailError::SelfMail);
    }
    if params.attachments.len() > MAX_ATTACHMENTS {
        return Err(MailError::TooManyAttachments);
    }

    let delivers_at = params.now + params.delivery_delay;
    let expires_at = delivers_at + params.expiry_days as u64 * 86400;

    Ok(Mail {
        id: params.id,
        sender: params.sender,
        recipient: params.recipient,
        subject: params.subject,
        body: params.body,
        attachments: params.attachments,
        gold: params.gold,
        cod: params.cod,
        sent_at: params.now,
        delivers_at,
        expires_at,
        read: false,
        collected: false,
    })
}

/// Result of collecting items/gold from a mail.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectResult {
    pub attachments: Vec<MailAttachment>,
    /// Gold received by the collector.
    pub gold: u32,
    /// COD amount paid by the collector (0 if no COD).
    pub cod_paid: u32,
    /// Original sender name to receive COD payment (None if no COD).
    pub cod_recipient: Option<String>,
}

/// Why collecting from a mail failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectError {
    NotFound,
    AlreadyCollected,
    /// Recipient doesn't have enough gold to pay the COD.
    NotEnoughGoldForCod {
        required: u32,
    },
}

/// A player's mailbox.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Mailbox {
    pub mails: Vec<Mail>,
}

impl Mailbox {
    /// Deliver a mail to the mailbox.
    pub fn deliver(&mut self, mail: Mail) {
        self.mails.push(mail);
    }

    /// Mark a mail as read.
    pub fn mark_read(&mut self, mail_id: u64) {
        if let Some(mail) = self.mails.iter_mut().find(|m| m.id == mail_id) {
            mail.read = true;
        }
    }

    /// Result of collecting from a mail.
    ///
    /// If the mail has a COD, the recipient must pay before items are released.
    /// The server sends the COD gold to the original sender via a new mail.
    pub fn collect(
        &mut self,
        mail_id: u64,
        recipient_gold: u32,
    ) -> Result<CollectResult, CollectError> {
        let mail = self
            .mails
            .iter_mut()
            .find(|m| m.id == mail_id)
            .ok_or(CollectError::NotFound)?;
        if mail.collected {
            return Err(CollectError::AlreadyCollected);
        }
        if mail.cod > 0 && recipient_gold < mail.cod {
            return Err(CollectError::NotEnoughGoldForCod { required: mail.cod });
        }

        mail.collected = true;
        mail.read = true;
        let attachments = std::mem::take(&mut mail.attachments);
        let gold = mail.gold;
        let cod_paid = mail.cod;
        let cod_recipient = if cod_paid > 0 {
            Some(mail.sender.clone())
        } else {
            None
        };
        mail.gold = 0;
        mail.cod = 0;

        Ok(CollectResult {
            attachments,
            gold,
            cod_paid,
            cod_recipient,
        })
    }

    /// Delete a mail (must be read and collected).
    pub fn delete(&mut self, mail_id: u64) -> bool {
        let can_delete = self
            .mails
            .iter()
            .any(|m| m.id == mail_id && m.read && m.collected);
        if can_delete {
            self.mails.retain(|m| m.id != mail_id);
        }
        can_delete
    }

    /// Remove expired mails. Returns expired mails (for return-to-sender).
    pub fn expire(&mut self, now: u64) -> Vec<Mail> {
        let (expired, remaining): (Vec<_>, Vec<_>) = self
            .mails
            .drain(..)
            .partition(|m| m.expires_at <= now && !m.collected);
        self.mails = remaining;
        expired
    }

    /// Count of unread mails.
    pub fn unread_count(&self) -> usize {
        self.mails.iter().filter(|m| !m.read).count()
    }

    /// Mails that have been delivered (delivery delay elapsed).
    pub fn delivered_mails(&self, now: u64) -> Vec<&Mail> {
        self.mails.iter().filter(|m| now >= m.delivers_at).collect()
    }

    /// Whether a specific mail is delivered and accessible.
    pub fn is_delivered(&self, mail_id: u64, now: u64) -> bool {
        self.mails
            .iter()
            .any(|m| m.id == mail_id && now >= m.delivers_at)
    }
}

/// Default mail expiry in days.
pub const MAIL_EXPIRY_DAYS: u32 = 30;

/// Create a return-to-sender mail from an expired mail.
///
/// The original sender becomes the recipient. Attachments and gold are
/// preserved. The returned mail has a new ID and fresh 30-day expiry.
/// Text-only mails (no attachments, no gold) are not returned.
pub fn return_to_sender(expired: &Mail, new_id: u64, now: u64) -> Option<Mail> {
    if expired.attachments.is_empty() && expired.gold == 0 {
        return None; // nothing to return
    }

    Some(Mail {
        id: new_id,
        sender: "System".to_string(),
        recipient: expired.sender.clone(),
        subject: format!("Returned: {}", expired.subject),
        body: String::new(),
        attachments: expired.attachments.clone(),
        gold: expired.gold,
        cod: 0,
        sent_at: now,
        delivers_at: now, // returned mail is instant
        expires_at: now + MAIL_EXPIRY_DAYS as u64 * 86400,
        read: false,
        collected: false,
    })
}

// --- Auction House mail ---

const AH_SENDER: &str = "Auction House";

/// AH cut on sold items (5%).
const AH_CUT_PERCENT: f32 = 0.05;

fn ah_mail(
    id: u64,
    recipient: &str,
    subject: String,
    gold: u32,
    attachments: Vec<MailAttachment>,
    now: u64,
) -> Mail {
    Mail {
        id,
        sender: AH_SENDER.to_string(),
        recipient: recipient.to_string(),
        subject,
        body: String::new(),
        attachments,
        gold,
        cod: 0,
        sent_at: now,
        delivers_at: now, // AH mail is instant
        expires_at: now + MAIL_EXPIRY_DAYS as u64 * 86400,
        read: false,
        collected: false,
    }
}

/// Create an AH mail for a successful sale (gold to seller, minus AH cut).
pub fn ah_sold_mail(id: u64, seller: &str, item_name: &str, sale_price: u32, now: u64) -> Mail {
    let cut = (sale_price as f32 * AH_CUT_PERCENT) as u32;
    let payout = sale_price.saturating_sub(cut);
    ah_mail(
        id,
        seller,
        format!("Auction successful: {item_name}"),
        payout,
        vec![],
        now,
    )
}

/// Create an AH mail for an expired auction (item returned to seller).
pub fn ah_expired_mail(id: u64, seller: &str, item_id: u32, count: u16, now: u64) -> Mail {
    ah_mail(
        id,
        seller,
        "Auction expired".into(),
        0,
        vec![MailAttachment { item_id, count }],
        now,
    )
}

/// Create an AH mail for an outbid notification (no attachments, just info).
pub fn ah_outbid_mail(id: u64, bidder: &str, item_name: &str, bid_refund: u32, now: u64) -> Mail {
    ah_mail(
        id,
        bidder,
        format!("Outbid on {item_name}"),
        bid_refund,
        vec![],
        now,
    )
}

/// Create an AH mail for a won auction (item to buyer).
pub fn ah_won_mail(id: u64, buyer: &str, item_id: u32, count: u16, now: u64) -> Mail {
    ah_mail(
        id,
        buyer,
        "Auction won".into(),
        0,
        vec![MailAttachment { item_id, count }],
        now,
    )
}

/// Calculate the AH cut from a sale price.
pub fn ah_cut(sale_price: u32) -> u32 {
    (sale_price as f32 * AH_CUT_PERCENT) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mail(id: u64) -> Mail {
        create_mail(NewMail {
            id,
            sender: "Alice".into(),
            recipient: "Bob".into(),
            subject: "Hello".into(),
            body: "Hi Bob!".into(),
            attachments: vec![MailAttachment {
                item_id: 100,
                count: 5,
            }],
            gold: 500,
            cod: 0,
            now: 1000,
            delivery_delay: 3600,
            expiry_days: 30,
        })
        .unwrap()
    }

    #[test]
    fn create_mail_success() {
        let mail = test_mail(1);
        assert_eq!(mail.sender, "Alice");
        assert_eq!(mail.recipient, "Bob");
        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.gold, 500);
        assert!(!mail.read);
        assert!(!mail.collected);
    }

    #[test]
    fn create_mail_self_rejected() {
        let result = create_mail(NewMail {
            id: 1,
            sender: "Alice".into(),
            recipient: "Alice".into(),
            subject: "".into(),
            body: "".into(),
            attachments: vec![],
            gold: 0,
            cod: 0,
            now: 0,
            delivery_delay: 0,
            expiry_days: 30,
        });
        assert_eq!(result, Err(MailError::SelfMail));
    }

    #[test]
    fn create_mail_too_many_attachments() {
        let attachments: Vec<_> = (0..13)
            .map(|i| MailAttachment {
                item_id: i,
                count: 1,
            })
            .collect();
        let result = create_mail(NewMail {
            id: 1,
            sender: "A".into(),
            recipient: "B".into(),
            subject: "".into(),
            body: "".into(),
            attachments,
            gold: 0,
            cod: 0,
            now: 0,
            delivery_delay: 0,
            expiry_days: 30,
        });
        assert_eq!(result, Err(MailError::TooManyAttachments));
    }

    #[test]
    fn send_cost_calculation() {
        assert_eq!(send_cost(0), 30);
        assert_eq!(send_cost(3), 120); // 30 + 3*30
    }

    #[test]
    fn mailbox_deliver_and_read() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        assert_eq!(mb.unread_count(), 1);
        mb.mark_read(1);
        assert_eq!(mb.unread_count(), 0);
    }

    #[test]
    fn mailbox_collect() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        let result = mb.collect(1, 9999).unwrap();
        assert_eq!(result.attachments.len(), 1);
        assert_eq!(result.gold, 500);
        assert!(mb.collect(1, 9999).is_err()); // already collected
    }

    #[test]
    fn mailbox_delete_requires_collected() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        assert!(!mb.delete(1)); // not read/collected
        mb.collect(1, 9999).unwrap();
        assert!(mb.delete(1));
        assert!(mb.mails.is_empty());
    }

    #[test]
    fn mailbox_expire() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1)); // expires_at = 1000 + 3600 + 30*86400
        let expired = mb.expire(99999999);
        assert_eq!(expired.len(), 1);
        assert!(mb.mails.is_empty());
    }

    #[test]
    fn mailbox_expire_keeps_collected() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        mb.collect(1, 9999).unwrap();
        let expired = mb.expire(99999999);
        assert!(expired.is_empty());
        assert_eq!(mb.mails.len(), 1);
    }

    // --- Delivery delay tests ---

    #[test]
    fn delay_items_1_hour() {
        assert_eq!(delivery_delay(true, false, false), 3600);
    }

    #[test]
    fn delay_gold_1_hour() {
        assert_eq!(delivery_delay(false, true, false), 3600);
    }

    #[test]
    fn delay_text_only_instant() {
        assert_eq!(delivery_delay(false, false, false), 0);
    }

    #[test]
    fn delay_same_account_instant() {
        assert_eq!(delivery_delay(true, true, true), 0);
    }

    #[test]
    fn mail_not_visible_before_delivery() {
        let mut mb = Mailbox::default();
        let mail = test_mail(1); // delivers_at = 1000 + 3600 = 4600
        mb.deliver(mail);

        assert!(mb.delivered_mails(2000).is_empty()); // before delivery
        assert!(!mb.is_delivered(1, 2000));

        assert_eq!(mb.delivered_mails(5000).len(), 1); // after delivery
        assert!(mb.is_delivered(1, 5000));
    }

    #[test]
    fn instant_mail_visible_immediately() {
        let mail = create_mail(NewMail {
            id: 2,
            sender: "Alice".into(),
            recipient: "Bob".into(),
            subject: "Hi".into(),
            body: "Text only".into(),
            attachments: vec![],
            gold: 0,
            cod: 0,
            now: 1000,
            delivery_delay: 0, // instant
            expiry_days: 30,
        })
        .unwrap();
        let mut mb = Mailbox::default();
        mb.deliver(mail);
        assert_eq!(mb.delivered_mails(1000).len(), 1);
    }

    // --- Return to sender tests ---

    #[test]
    fn return_expired_mail_with_items() {
        let original = test_mail(1); // has attachment + 500 gold
        let returned = return_to_sender(&original, 100, 99999).unwrap();
        assert_eq!(returned.recipient, "Alice"); // original sender
        assert_eq!(returned.sender, "System");
        assert_eq!(returned.subject, "Returned: Hello");
        assert_eq!(returned.attachments.len(), 1);
        assert_eq!(returned.gold, 500);
        assert_eq!(returned.delivers_at, 99999); // instant
        assert!(!returned.collected);
    }

    #[test]
    fn return_text_only_returns_none() {
        let text_mail = create_mail(NewMail {
            id: 2,
            sender: "A".into(),
            recipient: "B".into(),
            subject: "Hi".into(),
            body: "Text".into(),
            attachments: vec![],
            gold: 0,
            cod: 0,
            now: 1000,
            delivery_delay: 0,
            expiry_days: 30,
        })
        .unwrap();
        assert!(return_to_sender(&text_mail, 100, 99999).is_none());
    }

    #[test]
    fn expire_and_return_full_flow() {
        let mut sender_mb = Mailbox::default();
        let mut recipient_mb = Mailbox::default();

        // Send mail with items
        let mail = test_mail(1); // delivers_at=4600, expires_at=4600+30*86400
        recipient_mb.deliver(mail);

        // Expire after 30 days
        let expired = recipient_mb.expire(99999999);
        assert_eq!(expired.len(), 1);

        // Return to sender
        for m in &expired {
            if let Some(returned) = return_to_sender(m, 200, 99999999) {
                sender_mb.deliver(returned);
            }
        }
        assert_eq!(sender_mb.mails.len(), 1);
        assert_eq!(sender_mb.mails[0].recipient, "Alice");
    }

    #[test]
    fn returned_mail_has_30_day_expiry() {
        let original = test_mail(1);
        let now = 100000;
        let returned = return_to_sender(&original, 100, now).unwrap();
        assert_eq!(returned.expires_at, now + 30 * 86400);
    }

    // --- COD tests ---

    fn cod_mail(id: u64) -> Mail {
        create_mail(NewMail {
            id,
            sender: "Seller".into(),
            recipient: "Buyer".into(),
            subject: "Your purchase".into(),
            body: "".into(),
            attachments: vec![MailAttachment {
                item_id: 500,
                count: 1,
            }],
            gold: 0,
            cod: 10000, // 1 gold COD
            now: 1000,
            delivery_delay: 0,
            expiry_days: 3,
        })
        .unwrap()
    }

    #[test]
    fn cod_collect_pays_sender() {
        let mut mb = Mailbox::default();
        mb.deliver(cod_mail(1));
        let result = mb.collect(1, 50000).unwrap(); // has enough gold
        assert_eq!(result.cod_paid, 10000);
        assert_eq!(result.cod_recipient, Some("Seller".into()));
        assert_eq!(result.attachments.len(), 1);
    }

    #[test]
    fn cod_collect_not_enough_gold() {
        let mut mb = Mailbox::default();
        mb.deliver(cod_mail(1));
        let err = mb.collect(1, 5000).unwrap_err(); // not enough
        assert_eq!(err, CollectError::NotEnoughGoldForCod { required: 10000 });
    }

    #[test]
    fn no_cod_no_payment() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        let result = mb.collect(1, 0).unwrap();
        assert_eq!(result.cod_paid, 0);
        assert_eq!(result.cod_recipient, None);
    }

    // --- AH mail tests ---

    #[test]
    fn ah_sold_mail_with_cut() {
        let mail = ah_sold_mail(1, "Seller", "Thunderfury", 100000, 5000);
        assert_eq!(mail.sender, "Auction House");
        assert_eq!(mail.recipient, "Seller");
        assert_eq!(mail.gold, 95000); // 100000 - 5% cut
        assert!(mail.attachments.is_empty());
        assert_eq!(mail.delivers_at, 5000); // instant
    }

    #[test]
    fn ah_expired_returns_item() {
        let mail = ah_expired_mail(2, "Seller", 12345, 3, 5000);
        assert_eq!(mail.recipient, "Seller");
        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.attachments[0].item_id, 12345);
        assert_eq!(mail.attachments[0].count, 3);
        assert_eq!(mail.gold, 0);
    }

    #[test]
    fn ah_outbid_refunds_gold() {
        let mail = ah_outbid_mail(3, "Bidder", "Sword", 5000, 1000);
        assert_eq!(mail.recipient, "Bidder");
        assert_eq!(mail.gold, 5000);
        assert!(mail.attachments.is_empty());
    }

    #[test]
    fn ah_won_delivers_item() {
        let mail = ah_won_mail(4, "Buyer", 500, 1, 1000);
        assert_eq!(mail.recipient, "Buyer");
        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.attachments[0].item_id, 500);
    }

    #[test]
    fn ah_cut_calculation() {
        assert_eq!(ah_cut(100000), 5000); // 5%
        assert_eq!(ah_cut(0), 0);
    }
}
