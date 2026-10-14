use super::{banks, create_tables, guild_tabards as store};
use shared::guild_creation::{GuildTabardDesign, TABARD_COST};

fn design() -> GuildTabardDesign {
    GuildTabardDesign {
        icon: 1,
        icon_color: 2,
        border: 3,
        border_color: 4,
        background_color: 5,
    }
}

#[test]
fn guild_tabard_registration_preserves_legacy_guild_and_atomically_charges_on_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tabard.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (owner, _) = super::create_character(&db, 1, "Emblemowner", 1, 1).unwrap();
    super::set_character_guild_by_id(&db, owner, 7, "Raid Team").unwrap();
    let mut guild = banks::GuildData::default();
    guild.money = 81234;
    guild.ranks[2].name = "Raider".into();
    banks::save_guild(&db, 7, &guild);
    banks::save_guild_member_rank(&db, owner, 0);
    let mut record = super::inventory_store::load_inventory_record(&db, owner).unwrap();
    record.inventory.gold = u64::from(TABARD_COST) * 2;
    super::inventory_store::save_inventory_record(&db, owner, &record);
    assert_eq!(store::load_design(&db, 7), None);
    store::register(&db, owner, design()).unwrap();
    assert_eq!(
        super::load_inventory(&db, owner).gold,
        u64::from(TABARD_COST)
    );
    assert_eq!(banks::load_guild(&db, 7), guild);
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(store::load_design(&db, 7), Some(design()));
    assert_eq!(
        super::load_inventory(&db, owner).gold,
        u64::from(TABARD_COST)
    );
    assert_eq!(banks::load_guild(&db, 7), guild);
    store::register(&db, owner, design()).unwrap();
    assert_eq!(super::load_inventory(&db, owner).gold, 0);
    assert!(store::register(&db, owner, design()).is_err());
    assert_eq!(store::load_design(&db, 7), Some(design()));
}

#[test]
fn guild_tabard_registration_rejects_unguilded_nonleader_and_poor_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("reject.redb")).unwrap();
    create_tables(&db);
    let (member, _) = super::create_character(&db, 2, "Emblemmember", 1, 1).unwrap();
    assert!(store::register(&db, member, design()).is_err());
    super::set_character_guild_by_id(&db, member, 7, "Raid Team").unwrap();
    banks::save_guild(&db, 7, &banks::GuildData::default());
    banks::save_guild_member_rank(&db, member, 2);
    let mut record = super::inventory_store::load_inventory_record(&db, member).unwrap();
    record.inventory.gold = u64::from(TABARD_COST);
    super::inventory_store::save_inventory_record(&db, member, &record);
    assert!(store::register(&db, member, design()).is_err());
    assert_eq!(
        super::load_inventory(&db, member).gold,
        u64::from(TABARD_COST)
    );
    assert_eq!(store::load_design(&db, 7), None);
    banks::save_guild_member_rank(&db, member, 0);
    record.inventory.gold = u64::from(TABARD_COST) - 1;
    super::inventory_store::save_inventory_record(&db, member, &record);
    assert!(store::register(&db, member, design()).is_err());
    assert_eq!(
        super::load_inventory(&db, member).gold,
        u64::from(TABARD_COST) - 1
    );
    assert_eq!(store::load_design(&db, 7), None);
}
