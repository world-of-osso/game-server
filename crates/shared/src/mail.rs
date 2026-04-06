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

    /// Collect attachments and gold from a mail.
    /// Returns `(attachments, gold)` or `None` if already collected or not found.
    pub fn collect(&mut self, mail_id: u64) -> Option<(Vec<MailAttachment>, u32)> {
        let mail = self.mails.iter_mut().find(|m| m.id == mail_id)?;
        if mail.collected {
            return None;
        }
        mail.collected = true;
        mail.read = true;
        let attachments = std::mem::take(&mut mail.attachments);
        let gold = mail.gold;
        mail.gold = 0;
        Some((attachments, gold))
    }

    /// Delete a mail (must be read and collected).
    pub fn delete(&mut self, mail_id: u64) -> bool {
        let can_delete = self.mails.iter().any(|m| m.id == mail_id && m.read && m.collected);
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
            attachments: vec![MailAttachment { item_id: 100, count: 5 }],
            gold: 500,
            cod: 0,
            now: 1000,
            delivery_delay: 3600,
            expiry_days: 30,
        }).unwrap()
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
            id: 1, sender: "Alice".into(), recipient: "Alice".into(),
            subject: "".into(), body: "".into(), attachments: vec![],
            gold: 0, cod: 0, now: 0, delivery_delay: 0, expiry_days: 30,
        });
        assert_eq!(result, Err(MailError::SelfMail));
    }

    #[test]
    fn create_mail_too_many_attachments() {
        let attachments: Vec<_> = (0..13).map(|i| MailAttachment { item_id: i, count: 1 }).collect();
        let result = create_mail(NewMail {
            id: 1, sender: "A".into(), recipient: "B".into(),
            subject: "".into(), body: "".into(), attachments,
            gold: 0, cod: 0, now: 0, delivery_delay: 0, expiry_days: 30,
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
        let (items, gold) = mb.collect(1).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(gold, 500);
        assert!(mb.collect(1).is_none()); // already collected
    }

    #[test]
    fn mailbox_delete_requires_collected() {
        let mut mb = Mailbox::default();
        mb.deliver(test_mail(1));
        assert!(!mb.delete(1)); // not read/collected
        mb.collect(1);
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
        mb.collect(1);
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
        }).unwrap();
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
            id: 2, sender: "A".into(), recipient: "B".into(),
            subject: "Hi".into(), body: "Text".into(),
            attachments: vec![], gold: 0, cod: 0,
            now: 1000, delivery_delay: 0, expiry_days: 30,
        }).unwrap();
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
}
