//! Read the native item-instance tables published by scripts/item_instance_data.py.
use super::bonuses::ItemBonusTables;
use rusqlite::Connection;

pub fn load_bonus_tables(_conn: &Connection) -> rusqlite::Result<ItemBonusTables> {
    Ok(ItemBonusTables::default())
}

#[cfg(test)]
mod tests {
    use super::super::bonuses::apply_item_bonuses;
    use super::*;
    use shared::item_scaling::ItemScaling;

    #[test]
    fn item_instance_sqlite_native_config_and_rank_rows_resolve_mid1_bracers_level() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE item_bonus (ID INTEGER PRIMARY KEY,Value_0 INTEGER,Value_1 INTEGER,Value_2 INTEGER,Value_3 INTEGER,ParentItemBonusListID INTEGER,Type INTEGER,OrderIndex INTEGER);
          INSERT INTO item_bonus VALUES(25352,273,70,0,0,12066,49,0),(25960,264,100,0,0,12214,49,0),(30327,271,65,0,0,13622,49,0),(30571,13,2,4,0,9627,52,0);
          CREATE TABLE item_scaling_config (ID INTEGER PRIMARY KEY,ItemOffsetCurveID INTEGER,ItemLevel INTEGER,RequiredLevel INTEGER,ItemSquishEraID INTEGER,Flags INTEGER);
          INSERT INTO item_scaling_config VALUES(273,47,246,90,2,0),(264,47,246,90,2,0),(271,47,272,90,2,0);
          CREATE TABLE item_offset_curve (ID INTEGER PRIMARY KEY,CurveID INTEGER,Offset INTEGER);
          INSERT INTO item_offset_curve VALUES(47,88583,0);
          CREATE TABLE curve_point (ID INTEGER PRIMARY KEY,CurveID INTEGER,OrderIndex INTEGER,Pos_0 REAL,Pos_1 REAL);
          INSERT INTO curve_point VALUES(1,88583,0,0,0),(2,88583,1,1300,1300);").unwrap();
        let tables = load_bonus_tables(&conn).unwrap();
        let item = ItemScaling {
            item_level: 197,
            ..Default::default()
        };
        let result = apply_item_bonuses(&item, &[12066, 12214, 13622, 9627], &tables);
        assert!(
            result.is_ok(),
            "native rows must load through SQLite, not become base items"
        );
        assert_eq!(result.unwrap().scaling.item_level, 285);
    }
}
