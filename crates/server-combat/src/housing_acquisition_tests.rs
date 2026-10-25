use super::*;
use crate::persistence::housing_store::BUILD_KEY;
use rusqlite::{Connection, params};

pub(crate) fn collection_fixture() -> Connection {
    let db = crate::housing_catalog::tests::catalog_fixture();
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'HouseDecor', 1693, ?)",
        params![BUILD_KEY, r#"{"ItemID":"245259","ModelType":"1","InitialRotation_0":"0.0","InitialRotation_1":"0.0","InitialRotation_2":"0.0","InitialScale":"1.0"}"#],
    )
    .unwrap();
    // Sampled pinned source rows, including the zero player condition/extended cost.
    db.execute_batch("CREATE TABLE content_npc_vendor (entry INTEGER, item INTEGER, ExtendedCost INTEGER);
        INSERT INTO content_npc_vendor VALUES (252312,245259,0);
        CREATE TABLE item_x_item_effect (ItemID INTEGER, ItemEffectID INTEGER);
        INSERT INTO item_x_item_effect VALUES (245259,211701);
        CREATE TABLE item_effect (ID INTEGER, SpellID INTEGER, TriggerType INTEGER, PlayerConditionID INTEGER);
        INSERT INTO item_effect VALUES (211701,1238340,0,0);
        CREATE TABLE spell_effect (SpellID INTEGER, EffectIndex INTEGER, Effect INTEGER, EffectMiscValue_0 INTEGER, EffectBasePointsF REAL);
        INSERT INTO spell_effect VALUES (1238340,0,324,1693,0);").unwrap();
    db
}

#[test]
fn db2_follow_install_housing_collection_uses_declared_build() {
    let db = collection_fixture();
    let key = "2".repeat(32);
    db.execute(
        "UPDATE housing_catalog_build SET build_key=?, build='12.1.5.70391'",
        [&key],
    )
    .unwrap();
    db.execute("UPDATE housing_catalog_record SET build_key=?", [&key])
        .unwrap();
    let rules = read_collection_rules(&db).unwrap();
    assert_eq!(rules[&245259].decor_id, 1693);
}

#[test]
fn housing_collection_rules_resolve_each_authentic_item_and_do_not_require_a_vendor_acquisition() {
    let db = collection_fixture();
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'HouseDecor', 714, ?)",
        params![BUILD_KEY, r#"{"ItemID":"245284"}"#],
    )
    .unwrap();
    db.execute_batch("INSERT INTO item_x_item_effect VALUES (245284,211728); INSERT INTO item_effect VALUES (211728,1238378,0,0); INSERT INTO spell_effect VALUES (1238378,0,324,714,0);").unwrap();
    let rules = read_collection_rules(&db).unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[&245259].decor_id, 1693);
    assert_eq!(rules[&245284].decor_id, 714);
    db.execute(
        "UPDATE item_effect SET PlayerConditionID=100 WHERE SpellID=1238378",
        [],
    )
    .unwrap();
    let rules = read_collection_rules(&db).unwrap();
    assert_eq!(rules.len(), 1);
    assert!(!rules.contains_key(&245284));
    assert_eq!(rules[&245259].decor_id, 1693);
    db.execute("UPDATE housing_catalog_build SET build='wrong'", [])
        .unwrap();
    assert!(read_collection_rules(&db).is_err());
}

#[test]
fn housing_collection_rules_require_matching_pinned_item_and_collect_effect() {
    let db = collection_fixture();
    let rules = read_collection_rules(&db).unwrap();
    assert_eq!(rules[&245259].item_id, 245259);
    assert_eq!(rules[&245259].decor_id, 1693);
    db.execute("UPDATE spell_effect SET EffectMiscValue_0=80", [])
        .unwrap();
    assert!(!read_collection_rules(&db).unwrap().contains_key(&245259));
    db.execute("UPDATE spell_effect SET EffectMiscValue_0=1693", [])
        .unwrap();
    db.execute("UPDATE item_effect SET PlayerConditionID=100", [])
        .unwrap();
    assert!(!read_collection_rules(&db).unwrap().contains_key(&245259));
    db.execute("UPDATE item_effect SET PlayerConditionID=0", [])
        .unwrap();
    db.execute("UPDATE housing_catalog_build SET build='wrong'", [])
        .unwrap();
    assert!(read_collection_rules(&db).is_err());
}
