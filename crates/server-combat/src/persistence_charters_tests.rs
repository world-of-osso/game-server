use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::merchant::{VendorCatalog, VendorGoods};
use crate::persistence::{self, charters, create_tables};
use shared::protocol::GUILD_CHARTER_ITEM;

fn catalog() -> (ItemTemplates, VendorCatalog) {
    // Authentic content_item5863: quality1, stack1, pickup-bound, zero item buy/sell
    // prices. The registrar's1000c price is realm policy, not a fabricated vendor row.
    let mut templates = ItemTemplates::default();
    templates.retail.insert(
        GUILD_CHARTER_ITEM,
        ItemTemplate {
            max_stack: 1,
            bonding: 1,
            allowable_class: -1,
            allowable_race: u64::MAX,
            ..Default::default()
        },
    );
    let mut catalog = VendorCatalog::default();
    catalog.goods.insert(
        GUILD_CHARTER_ITEM,
        VendorGoods {
            name: "Guild Charter".into(),
            quality: 1,
            buy_price: 0,
            sell_price: 0,
            buy_count: 1,
            max_count: 1,
        },
    );
    (templates, catalog)
}

fn character(db: &redb::Database, account: u64, name: &str, gold: u64) -> u64 {
    let (id, _) = persistence::create_character(db, account, name, 1, 1).unwrap();
    let mut record = persistence::inventory_store::load_inventory_record(db, id).unwrap();
    record.inventory.gold = gold;
    persistence::inventory_store::save_inventory_record(db, id, &record);
    id
}

#[test]
fn guild_charter_purchase_sign_rename_create_and_consumption_survive_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("charter.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (templates, catalog) = catalog();
    let owner = character(&db, 1, "Charowner", 5000);
    let signers = ["Sigone", "Sigtwo", "Sigthree", "Sigfour"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| character(&db, i as u64 + 2, name, 5000))
        .collect::<Vec<_>>();
    let guid = charters::purchase(&db, &templates, &catalog, owner, "First Guild", 100).unwrap();
    assert_eq!(persistence::load_inventory(&db, owner).gold, 4000);
    assert_eq!(
        persistence::load_inventory(&db, owner).items[0].item_id,
        GUILD_CHARTER_ITEM
    );
    assert!(charters::turn_in(&db, &templates, guid, owner, 200).is_err());
    charters::sign(&db, guid, signers[0], 2).unwrap();
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(
        charters::snapshot(&db, guid, owner, 1).unwrap().signatures,
        vec!["Sigone"]
    );
    charters::rename(&db, guid, owner, "Durable Guild").unwrap();
    for (i, signer) in signers.iter().enumerate().skip(1) {
        charters::sign(&db, guid, *signer, i as u64 + 2).unwrap();
    }
    let created = charters::turn_in(&db, &templates, guid, owner, 300).unwrap();
    assert_eq!(created.guild_name, "Durable Guild");
    assert_eq!(created.founding_members, signers);
    let expected_joins = ["Charowner", "Sigone", "Sigtwo", "Sigthree", "Sigfour"]
        .into_iter()
        .map(|name| shared::protocol::GuildEvent {
            timestamp: 300,
            kind: shared::protocol::GuildEventKind::Join {
                member: name.into(),
            },
        })
        .collect::<Vec<_>>();
    assert_eq!(
        crate::guild_event_log::query(&db, owner).events,
        expected_joins
    );
    assert!(persistence::load_inventory(&db, owner).items.is_empty());
    assert!(charters::owned_charter_guid(&db, owner).is_none());
    assert!(charters::snapshot(&db, guid, owner, 1).is_err());
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(
        crate::guild_event_log::query(&db, owner).events,
        expected_joins
    );
    let guild = persistence::banks::load_guild(&db, created.guild_id);
    assert_eq!(
        persistence::banks::load_guild_member_rank(&db, &guild, owner),
        0
    );
    for signer in signers {
        assert_eq!(
            persistence::load_character_guild(&db, signer)
                .unwrap()
                .guild_id,
            created.guild_id
        );
        assert_eq!(
            persistence::banks::load_guild_member_rank(&db, &guild, signer),
            guild.lowest_rank()
        );
    }
    assert_eq!(charters::created_at(&db, created.guild_id), Some(300));
    assert!(charters::turn_in(&db, &templates, guid, owner, 400).is_err());
}

#[test]
fn guild_charter_permissions_unique_signer_accounts_and_pending_invites_are_authoritative() {
    let db = persistence::temp_database();
    let (templates, catalog) = catalog();
    let owner = character(&db, 1, "Owner", 5000);
    let signer = character(&db, 2, "Signer", 5000);
    let alt = character(&db, 2, "Signalt", 5000);
    let invitee = character(&db, 3, "Invitee", 5000);
    let guid = charters::purchase(&db, &templates, &catalog, owner, "Signatures", 100).unwrap();
    assert!(charters::sign(&db, guid, owner, 1).is_err());
    charters::sign(&db, guid, signer, 2).unwrap();
    assert!(charters::sign(&db, guid, alt, 2).is_err());
    assert!(charters::rename(&db, guid, signer, "Stolen").is_err());
    assert!(charters::turn_in(&db, &templates, guid, signer, 100).is_err());
    persistence::set_pending_guild_invite(&db, invitee, 9, "Existing", "Inviter");
    assert!(charters::sign(&db, guid, invitee, 3).is_err());
    assert_eq!(
        charters::snapshot(&db, guid, owner, 1).unwrap().signatures,
        vec!["Signer"]
    );
    assert_eq!(persistence::load_inventory(&db, owner).gold, 4000);
    assert_eq!(persistence::load_inventory(&db, owner).items.len(), 1);
}

#[test]
fn guild_charter_refused_purchase_and_destroyed_charter_do_not_leave_durable_phantom_state() {
    let db = persistence::temp_database();
    let (templates, catalog) = catalog();
    let owner = character(&db, 1, "Poorowner", 999);
    assert!(charters::purchase(&db, &templates, &catalog, owner, "Poor Guild", 100).is_err());
    assert_eq!(persistence::load_inventory(&db, owner).gold, 999);
    assert!(charters::owned_charter_guid(&db, owner).is_none());
    let rich = character(&db, 2, "Richowner", 5000);
    let guid = charters::purchase(&db, &templates, &catalog, rich, "Rich Guild", 100).unwrap();
    let mut model = crate::inventory::load_model(&db, &templates, rich).unwrap();
    let location = model
        .view()
        .into_iter()
        .find(|(_, item)| item.item_guid == guid)
        .unwrap()
        .0;
    model.destroy(location, 1).unwrap();
    persistence::inventory_store::save_inventory_record(&db, rich, &model.into_record());
    assert!(charters::owned_charter_guid(&db, rich).is_none());
    assert!(charters::snapshot(&db, guid, rich, 2).is_err());
    assert!(charters::purchase(&db, &templates, &catalog, rich, "Replacement", 200).is_ok());
}

#[test]
fn guild_charter_signature_is_removed_atomically_when_signer_accepts_an_invite() {
    let db = persistence::temp_database();
    let (templates, catalog) = catalog();
    let owner = character(&db, 1, "Owner", 5000);
    let signer = character(&db, 2, "Signer", 5000);
    let guid = charters::purchase(&db, &templates, &catalog, owner, "Founders", 100).unwrap();
    charters::sign(&db, guid, signer, 2).unwrap();
    persistence::set_pending_guild_invite(&db, signer, 9, "Elsewhere", "Inviter");
    persistence::accept_pending_guild_invite(&db, signer).unwrap();
    assert!(
        charters::snapshot(&db, guid, owner, 1)
            .unwrap()
            .signatures
            .is_empty()
    );
}
