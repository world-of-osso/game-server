use super::*;

#[test]
fn guardian51_level_selection_preserves_flags_and_signed_delta() {
    use crate::world_data::summon_levels::GuardianLevelRange;
    let inputs = |range, delta| GuardianLevelRange { range, delta };
    assert_eq!(roll_guardian_level(20, inputs((1, 30), (0, 0)), 0), 20);
    assert_eq!(roll_guardian_level(80, inputs((1, 30), (0, 0)), 0), 30);
    assert_eq!(roll_guardian_level(1, inputs((5, 30), (2, 2)), 0), 7);
    assert_eq!(roll_guardian_level(80, inputs((5, 30), (-2, -2)), 0), 28);
    assert_eq!(
        roll_guardian_level(20, inputs((1, 30), (2, 2)), USE_CREATURE_LEVEL),
        32
    );
    assert_eq!(
        roll_guardian_level(20, inputs((1, 30), (0, 0)), ONLY_VISIBLE_TO_SUMMONER),
        20
    );
    assert_eq!(roll_guardian_level(80, inputs((0, 0), (0, 0)), 0), 0);
    assert_eq!(
        roll_guardian_level(80, inputs((0, 0), (0, 0)), USE_CREATURE_LEVEL),
        1
    );
    for _ in 0..64 {
        let level = roll_guardian_level(80, inputs((1, 30), (3, -2)), 0);
        assert!(
            (28..=33).contains(&level),
            "source sorted inclusive delta {level}"
        );
    }
}

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
