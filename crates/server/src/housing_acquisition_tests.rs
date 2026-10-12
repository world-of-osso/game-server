use super::*;
use crate::persistence::housing_store::BUILD_KEY;
use rusqlite::{Connection, params};

pub(crate) fn collection_fixture() -> Connection {
    let db = crate::housing_catalog::tests::catalog_fixture();
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'HouseDecor', 1693, ?)",
        params![BUILD_KEY, r#"{"ItemID":"245259"}"#],
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
fn housing_collection_rule_requires_matching_pinned_item_vendor_and_collect_effect() {
    let db = collection_fixture();
    let rule = read_bookcase_rule(&db).unwrap();
    assert_eq!(rule.item_id, 245259);
    assert_eq!(rule.decor_id, 1693);
    db.execute("UPDATE spell_effect SET EffectMiscValue_0=80", [])
        .unwrap();
    assert!(read_bookcase_rule(&db).is_err());
    db.execute("UPDATE spell_effect SET EffectMiscValue_0=1693", [])
        .unwrap();
    db.execute("UPDATE item_effect SET PlayerConditionID=100", [])
        .unwrap();
    assert!(read_bookcase_rule(&db).is_err());
    db.execute("UPDATE item_effect SET PlayerConditionID=0", [])
        .unwrap();
    db.execute("UPDATE housing_catalog_build SET build='wrong'", [])
        .unwrap();
    assert!(read_bookcase_rule(&db).is_err());
}
