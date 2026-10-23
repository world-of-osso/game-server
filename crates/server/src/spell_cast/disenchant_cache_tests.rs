//! Complete read-only definitions remain reusable without sharing live resources.
use super::*;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

fn write_source(path: &Path) {
    Connection::open(path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE content_item (
                ID INTEGER, ClassID INTEGER, SubclassID INTEGER, Quality INTEGER, ItemLevel INTEGER);
             CREATE TABLE item_sparse (
                ID INTEGER, ExpansionID INTEGER, Flags_0 INTEGER, Bonding INTEGER,
                Stackable INTEGER, ZoneBound_0 INTEGER, ZoneBound_1 INTEGER,
                InstanceBound INTEGER, SellPrice INTEGER);
             CREATE TABLE item_disenchant_loot (
                ID INTEGER, Class INTEGER, Subclass INTEGER, Quality INTEGER,
                MinLevel INTEGER, MaxLevel INTEGER, ExpansionID INTEGER, SkillRequired INTEGER);
             CREATE TABLE tdb_disenchant_loot_template (
                Entry INTEGER, Item INTEGER, ItemType INTEGER, Chance REAL, QuestRequired INTEGER,
                GroupId INTEGER, MinCount INTEGER, MaxCount INTEGER, LootMode INTEGER);
             CREATE TABLE tdb_reference_loot_template AS
                SELECT * FROM tdb_disenchant_loot_template;
             INSERT INTO content_item VALUES (101,2,1,2,10),(202,2,1,2,10);
             INSERT INTO item_sparse VALUES (101,0,0,0,1,0,0,0,50),(202,0,32768,0,1,0,0,0,50);
             INSERT INTO item_disenchant_loot VALUES (7,2,-1,2,1,20,-2,25),(8,2,-1,2,1,20,-2,50);
             INSERT INTO tdb_disenchant_loot_template VALUES (7,500,0,100,0,0,2,2,1);
             INSERT INTO tdb_reference_loot_template VALUES (9,501,0,100,0,0,1,1,1);",
        )
        .unwrap();
}

fn readonly(path: &Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}

#[test]
fn readonly_definitions_survive_revoked_sql_reads_and_independent_app_edits() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_source(&path);
    let original = load_data(&readonly(&path)).unwrap();
    let reused_reader = readonly(&path);
    reused_reader.authorizer(Some(|context: AuthContext<'_>| match context.action {
        AuthAction::Read { .. } => Authorization::Deny,
        _ => Authorization::Allow,
    }));
    let reused =
        load_data(&reused_reader).expect("warm readonly DisenchantData must not query revoked SQL");
    let mut first = App::new();
    first.insert_resource(original);
    let mut second = App::new();
    second.insert_resource(reused);
    {
        let mut local = first.world_mut().resource_mut::<DisenchantData>();
        local.items.get_mut(&101).unwrap().skill = 999;
        local.loot.disenchant.get_mut(&7).unwrap().entries[0].item = 999;
        local.loot.reference.clear();
    }
    let sibling = second.world().resource::<DisenchantData>();
    assert_eq!(
        (sibling.items[&101].loot_id, sibling.items[&101].skill),
        (7, 25)
    );
    assert!(!sibling.items.contains_key(&202));
    assert_eq!(sibling.loot.disenchant[&7].entries[0].item, 500);
    assert_eq!(sibling.loot.reference[&9].entries[0].item, 501);
    let warm = load_data(&reused_reader).unwrap();
    assert_eq!(warm.items[&101].skill, 25);
    assert_eq!(warm.loot.disenchant[&7].entries[0].min_count, 2);
    assert_eq!(warm.loot.reference[&9].entries[0].item, 501);
}

#[test]
fn writable_edits_and_new_readonly_content_do_not_reuse_old_definitions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    write_source(&path);
    let reader = readonly(&path);
    let original = load_data(&reader).unwrap();
    let writer = Connection::open(&path).unwrap();
    writer
        .execute_batch(
            "BEGIN;
             UPDATE item_disenchant_loot SET SkillRequired=80 WHERE ID=7;
             UPDATE tdb_disenchant_loot_template SET Item=555 WHERE Entry=7;",
        )
        .unwrap();
    let uncommitted = load_data(&writer).unwrap();
    assert_eq!(uncommitted.items[&101].skill, 80);
    assert_eq!(uncommitted.loot.disenchant[&7].entries[0].item, 555);
    writer
        .execute_batch(
            "ROLLBACK;
             UPDATE item_disenchant_loot SET SkillRequired=75 WHERE ID=7;
             UPDATE tdb_disenchant_loot_template SET Item=556 WHERE Entry=7;",
        )
        .unwrap();
    let changed = load_data(&reader).unwrap();
    assert_eq!(changed.items[&101].skill, 75);
    assert_eq!(changed.loot.disenchant[&7].entries[0].item, 556);
    assert_eq!(original.items[&101].skill, 25);
    assert_eq!(original.loot.disenchant[&7].entries[0].item, 500);
}
