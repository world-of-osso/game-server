use super::*;
use crate::persistence::{create_character_with_appearance, temp_database};
use shared::components::CharacterAppearance;

#[test]
fn missing_authored_start_fails_creation_without_persisting_a_character() {
    let db = temp_database();
    let mut kit = OwnedKit::bare();
    kit.create_info.2.remove(&(6, 1));
    let error = create_character_with_appearance(
        &db,
        1,
        "Mulgore",
        6,
        1,
        CharacterAppearance::default(),
        &kit.kit(),
    )
    .unwrap_err();
    assert!(
        error.contains("race 6 class 1 has no authored playercreateinfo start"),
        "{error}"
    );
    assert!(crate::persistence::list_characters_for_account(&db, 1).is_empty());
    assert_eq!(
        crate::persistence::find_character_id_by_name(&db, "Mulgore"),
        None
    );
}

#[test]
fn starts_use_both_race_and_class_and_preserve_source_facing_in_homebind() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE tdb_playercreateinfo (race, class, map, position_x, position_y, position_z, orientation);
         INSERT INTO tdb_playercreateinfo VALUES
         (1,1,0,-8914.57,-133.909,80.5378,5.10444),
         (1,6,609,2355.84,-5664.77,426.028,3.65997);",
    ).unwrap();
    let mut kit = OwnedKit::bare();
    kit.create_info.2 = crate::player_start::load_starts(&conn).unwrap();
    let db = temp_database();
    for (class, name, expected) in [
        (1, "Warrior", (0, -8914.57, -133.909, 80.5378, 5.10444)),
        (6, "Knight", (609, 2355.84, -5664.77, 426.028, 3.65997)),
    ] {
        let (id, created) = create_character_with_appearance(
            &db,
            1,
            name,
            1,
            class,
            CharacterAppearance::default(),
            &kit.kit(),
        )
        .unwrap();
        assert_eq!(
            (created.x, created.y, created.z),
            (expected.1, expected.2, expected.3)
        );
        assert_eq!(
            crate::maps::persistence::load_character_map(&db, id),
            expected.0
        );
        let bind = crate::spell_cast::homebind::load_bind(&db, id).unwrap();
        assert_eq!(bind.0.orientation, expected.4);
        assert_eq!(
            bind.0.position,
            Vec3::new(expected.1, expected.2, expected.3)
        );
    }
}

#[test]
fn demon_hunter_unimplemented_intro_uses_authored_race_start_but_missing_rows_do_not() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE tdb_playercreateinfo (race, class, map, position_x, position_y, position_z, orientation);
         INSERT INTO tdb_playercreateinfo VALUES
         (4,1,1,10311.3,831.463,1326.57,5.48033),
         (4,12,1481,1180.64,3284.96,70.3484,4.76134),
         (10,1,530,10349.6,-6357.29,33.4026,5.31605);",
    ).unwrap();
    let starts = crate::player_start::load_starts(&conn).unwrap();
    assert_eq!(starts.get(&(4, 12)), starts.get(&(4, 1)));
    assert!(
        !starts.contains_key(&(10, 12)),
        "a missing DH row must not use its Warrior row"
    );
    assert_eq!(starts[&(95, 1)].map_id, 2991);
    assert_eq!(starts[&(52, 13)].map_id, 0);
}
