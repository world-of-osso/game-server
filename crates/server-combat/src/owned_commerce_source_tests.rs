//! Source identity across production commerce transactions and redb reopen.
//! Catalog rows come from the existing importer/regeneration fixture, not invented templates.

use redb::Database;
use rusqlite::Connection;
use shared::item_data::ItemDefinitionSource::{self, Forever70205, Retail};
use shared::protocol::{ItemStack, MailAction, SendMail};

use crate::inventory::load_model;
use crate::item_templates::{ItemTemplates, load_runtime_item_templates};
use crate::persistence::inventory_store::{
    load_inventory_record, load_item_definition_source, save_inventory_record,
};
use crate::persistence::{self, InventoryItemData};

#[path = "skyborne_kit_fixture.rs"]
mod fixture;

pub(crate) struct Catalog {
    _directory: tempfile::TempDir,
    conn: Connection,
    pub templates: ItemTemplates,
}

impl Catalog {
    pub fn load() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(fixture::SQL).unwrap();
        let templates = load_runtime_item_templates(&path).unwrap();
        Self {
            _directory: directory,
            conn,
            templates,
        }
    }

    /// Controlled ownership/counts; all item metadata is copied from captured source rows.
    pub fn seed(
        &self,
        db: &Database,
        character: u64,
        items: &[(ItemDefinitionSource, u32, u32)],
    ) -> Vec<u64> {
        let mut record = load_inventory_record(db, character).unwrap();
        record.inventory.items.clear();
        record.inventory.gold = 10_000;
        record.placements.clear();
        record.equipped.clear();
        record.bag_slots.clear();
        record.instances.clear();
        record.definition_sources.clear();
        let mut guids = Vec::new();
        for &(source, id, count) in items {
            let guid = persistence::next_id(db, "item_instance");
            let table = match source {
                Retail => "content_item",
                Forever70205 => "skyborne_content_item",
            };
            let (name, quality, sell_price) = self
                .conn
                .query_row(
                    &format!("SELECT Name, Quality, SellPrice FROM {table} WHERE ID = ?1"),
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            let template = self.templates.get(source, id).unwrap();
            assert!(count > 0 && count <= template.max_stack);
            record.inventory.items.push(InventoryItemData {
                item_guid: guid,
                item_id: id,
                name,
                quality,
                required_level: template.required_level,
                stack_count: count,
                vendor_sell_price: sell_price,
                unique_auction: false,
            });
            record.definition_sources.insert(guid, source);
            guids.push(guid);
        }
        save_inventory_record(db, character, &record);
        guids
    }
}

pub(crate) fn character(db: &Database, name: &str) -> u64 {
    persistence::create_character_with_test_bags(db, 1, name, 1, 1)
        .unwrap()
        .0
}

pub(crate) fn assert_stacks(
    stacks: impl IntoIterator<Item = ItemStack>,
    expected: &[(u64, u32, ItemDefinitionSource, u32)],
) {
    let mut actual: Vec<_> = stacks
        .into_iter()
        .map(|item| {
            (
                item.item_guid,
                item.item_id,
                item.definition_source,
                item.count,
            )
        })
        .collect();
    let mut expected = expected.to_vec();
    actual.sort_by_key(|item| item.0);
    expected.sort_by_key(|item| item.0);
    assert_eq!(actual, expected);
}

pub(crate) fn assert_owned(
    db: &Database,
    catalog: &Catalog,
    owner: u64,
    expected: &[(u64, u32, ItemDefinitionSource, u32)],
) {
    let model = load_model(db, &catalog.templates, owner).unwrap();
    assert_stacks(model.view().into_values(), expected);
    assert_eq!(
        persistence::load_inventory(db, owner).items.len(),
        expected.len()
    );
    for &(guid, _, source, _) in expected {
        assert_eq!(load_item_definition_source(db, guid), source);
    }
}

#[test]
fn owned_commerce_sources_mail_return_and_take_preserve_both_catalogs_after_reopen() {
    let catalog = Catalog::load();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let sender = character(&db, "Sender");
    let recipient = character(&db, "Recipient");
    let guids = catalog.seed(&db, sender, &[(Retail, 2947, 1), (Forever70205, 2947, 1)]);
    catalog.seed(&db, recipient, &[]);
    let expected = [
        (guids[0], 2947, Retail, 1),
        (guids[1], 2947, Forever70205, 1),
    ];
    let request = SendMail {
        object: 0,
        recipient: "Recipient".into(),
        subject: "Collision parcel".into(),
        body: String::new(),
        attachments: guids.clone(),
        money: 0,
        cod: 0,
    };
    let now = crate::mail::unix_now();
    crate::mail::send_mail(&db, &catalog.templates, sender, &request, now).unwrap();
    assert_owned(&db, &catalog, sender, &[]);
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let contents = crate::mail::mailbox_contents(&db, &catalog.templates, recipient, 0);
    assert_eq!(contents.mails.len(), 1);
    assert_stacks(
        contents.mails[0].attachments.iter().map(|a| a.item.clone()),
        &expected,
    );
    crate::mail::act_on_mail(
        &db,
        &catalog.templates,
        recipient,
        contents.mails[0].mail_id,
        MailAction::Return,
        now,
    )
    .unwrap();
    assert!(persistence::mail_store::load_mailbox(&db, recipient).is_empty());
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let returned = crate::mail::mailbox_contents(&db, &catalog.templates, sender, 0);
    assert_eq!(returned.mails.len(), 1);
    assert!(returned.mails[0].returned);
    assert_stacks(
        returned.mails[0].attachments.iter().map(|a| a.item.clone()),
        &expected,
    );
    for slot in 0..2 {
        crate::mail::act_on_mail(
            &db,
            &catalog.templates,
            sender,
            returned.mails[0].mail_id,
            MailAction::TakeAttachment { slot },
            now,
        )
        .unwrap();
    }
    assert_owned(&db, &catalog, sender, &expected);
    crate::mail::send_mail(&db, &catalog.templates, sender, &request, now).unwrap();
    drop(db);

    let db = Database::open(file.path()).unwrap();
    let mail = persistence::mail_store::load_mailbox(&db, recipient)
        .pop()
        .unwrap();
    for slot in 0..2 {
        crate::mail::act_on_mail(
            &db,
            &catalog.templates,
            recipient,
            mail.mail_id,
            MailAction::TakeAttachment { slot },
            now,
        )
        .unwrap();
    }
    assert!(
        persistence::mail_store::load_mailbox(&db, recipient)[0]
            .items
            .is_empty()
    );
    drop(db);

    let db = Database::open(file.path()).unwrap();
    assert_owned(&db, &catalog, sender, &[]);
    assert_owned(&db, &catalog, recipient, &expected);
    let mut model = load_model(&db, &catalog.templates, recipient).unwrap();
    let view = model.view();
    let location = |guid| {
        *view
            .iter()
            .find(|(_, item)| item.item_guid == guid)
            .unwrap()
            .0
    };
    assert_eq!(
        model.auto_equip(location(guids[0])).unwrap_err().reason,
        shared::protocol::InventoryErrorReason::NotEquippable
    );
    model.auto_equip(location(guids[1])).unwrap();
    assert_eq!(
        model.equipment_snapshot().items[0].item.definition_source,
        Forever70205
    );
}
