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

    let expires_at = params.now + params.delivery_delay + params.expiry_days as u64 * 86400;

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
        mb.collect(1); // collected mails don't expire
        let expired = mb.expire(99999999);
        assert!(expired.is_empty());
        assert_eq!(mb.mails.len(), 1);
    }
}
