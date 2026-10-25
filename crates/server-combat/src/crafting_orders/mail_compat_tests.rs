use crate::persistence::InventoryItemData;
use crate::persistence::mail_store::*;
use redb::TableDefinition;

#[derive(bitcode::Encode)]
enum OldSender {
    Player { character_id: u64, name: String },
    AuctionHouse,
}
#[derive(bitcode::Encode)]
struct OldMail {
    mail_id: u64,
    sender: OldSender,
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

#[test]
fn crafting_orders_old_mailboxes_survive_sender_extension_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mail.redb");
    let db = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&db);
    let old: Vec<_> = (0..24)
        .map(|i| OldMail {
            mail_id: i,
            sender: if i % 2 == 0 {
                OldSender::Player {
                    character_id: 8,
                    name: "Sender".into(),
                }
            } else {
                OldSender::AuctionHouse
            },
            subject: "Old mail".into(),
            body: String::new(),
            items: Vec::new(),
            money: i * 100,
            cod: 0,
            delivers_at: 100,
            expires_at: 200,
            read: false,
            returned: false,
            cod_payment: false,
        })
        .collect();
    let txn = db.begin_write().unwrap();
    txn.open_table(TableDefinition::<u64, &[u8]>::new("character_mail"))
        .unwrap()
        .insert(9, bitcode::encode(&old).as_slice())
        .unwrap();
    txn.commit().unwrap();
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    crate::persistence::create_tables(&db);
    let mails = load_mailbox(&db, 9);
    assert_eq!(mails.len(), 24);
    for (i, mail) in mails.iter().enumerate() {
        assert_eq!(mail.money, i as u64 * 100);
        assert_eq!(
            mail.sender,
            if i % 2 == 0 {
                MailSender::Player {
                    character_id: 8,
                    name: "Sender".into(),
                }
            } else {
                MailSender::AuctionHouse
            }
        );
    }
    assert_eq!(MailSender::CraftingOrder.name(), "Crafting Order");
    let mail = MailData {
        mail_id: 30,
        sender: MailSender::CraftingOrder,
        subject: "Result".into(),
        body: String::new(),
        items: vec![MailItemData {
            slot: 0,
            item: InventoryItemData {
                item_guid: 1,
                item_id: 22658,
                name: "Cloak".into(),
                quality: 4,
                required_level: 60,
                stack_count: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            },
        }],
        money: 0,
        cod: 0,
        delivers_at: 100,
        expires_at: 200,
        read: false,
        returned: false,
        cod_payment: false,
    };
    assert!(!mail.returnable());
    deliver_mail(&db, 9, mail);
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    crate::persistence::create_tables(&db);
    let mails = load_mailbox(&db, 9);
    assert_eq!(mails.len(), 25);
    assert_eq!(mails[24].sender, MailSender::CraftingOrder);
}
