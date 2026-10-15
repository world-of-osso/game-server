use super::*;

#[test]
fn toyfx28_sourced_signed_title_sentinel_does_not_abort_other_summon_properties() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE summon_properties (ID INTEGER, Control INTEGER, Faction INTEGER, Title INTEGER, Slot INTEGER, Flags_0 INTEGER);
        INSERT INTO summon_properties VALUES (3001,1,0,-1,0,4610),(4493,1,0,2,0,67592);").unwrap();
    let properties = load_summon_properties(&conn).unwrap();
    assert_eq!(i64::from(properties.0[&3001].title), -1);
    assert!(!properties.0[&3001].is_supported());
    assert!(properties.0[&4493].is_supported());
    assert!(!properties.0[&4493].is_pet());
}
