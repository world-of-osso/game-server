//! The third MailSender variant changes bitcode's enum packing. Decode v1 with
//! its exact two-variant ABI and atomically replace it with character_mail_v2.
use super::{MailData, MailItemData, MailSender, push_mail};
use redb::{Database, ReadableTable, TableDefinition};

const V1: TableDefinition<u64, &[u8]> = TableDefinition::new("character_mail");

#[derive(bitcode::Decode)]
enum SenderV1 {
    Player { character_id: u64, name: String },
    AuctionHouse,
}

#[derive(bitcode::Decode)]
struct MailV1 {
    mail_id: u64,
    sender: SenderV1,
    subject: String,
    body: String,
    items: Vec<MailItemData>,
    money: u64,
    cod: u64,
    delivers_at: u64,
    expires_at: u64,
    read: bool,
    returned: bool,
    cod_payment: bool,
}

impl From<MailV1> for MailData {
    fn from(mail: MailV1) -> Self {
        Self {
            mail_id: mail.mail_id,
            sender: match mail.sender {
                SenderV1::Player { character_id, name } => {
                    MailSender::Player { character_id, name }
                }
                SenderV1::AuctionHouse => MailSender::AuctionHouse,
            },
            subject: mail.subject,
            body: mail.body,
            items: mail.items,
            money: mail.money,
            cod: mail.cod,
            delivers_at: mail.delivers_at,
            expires_at: mail.expires_at,
            read: mail.read,
            returned: mail.returned,
            cod_payment: mail.cod_payment,
        }
    }
}

pub(crate) fn migrate_sender_mailboxes(db: &Database) {
    let txn = db.begin_write().expect("begin mailbox sender migration");
    let rows: Vec<(u64, Vec<MailV1>)> = {
        let table = txn.open_table(V1).expect("open v1 mailboxes");
        table
            .iter()
            .expect("iterate v1 mailboxes")
            .map(|row| {
                let (id, value) = row.expect("read v1 mailbox row");
                let mails = bitcode::decode(value.value())
                    .unwrap_or_else(|error| panic!("decode v1 mailbox {}: {error}", id.value()));
                (id.value(), mails)
            })
            .collect()
    };
    for (id, mails) in rows {
        for mail in mails {
            push_mail(&txn, id, mail.into());
        }
    }
    txn.delete_table(V1).expect("drop v1 mailbox table");
    crate::persistence_commit::commit(txn).expect("commit mailbox sender migration");
}
