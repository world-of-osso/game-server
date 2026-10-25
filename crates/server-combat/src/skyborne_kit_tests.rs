//! Exact importer/regeneration fixture through OwnedKit and redb close/reopen.

use super::*;
use crate::persistence::{
    create_character_with_appearance, inventory_store::load_inventory_record,
};
use shared::components::CharacterAppearance;

#[path = "skyborne_kit_fixture.rs"]
mod fixture;

fn fixture_path() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(fixture::SQL).unwrap();
    (directory, path)
}

fn load_kit(path: &Path) -> OwnedKit {
    let conn = Connection::open(path).unwrap();
    OwnedKit {
        create_info: load_player_create_infos(&conn).unwrap(),
        templates: crate::item_templates::load_runtime_item_templates(path).unwrap(),
        catalog: crate::merchant::load_vendor_catalog(&conn).unwrap(),
    }
}

fn expected_items(class: u8) -> Vec<(u32, u32)> {
    match class {
        1 => vec![
            (117, 4),
            (6948, 1),
            (271671, 1),
            (271672, 1),
            (271669, 1),
            (280399, 1),
            (876, 1),
        ],
        3 => vec![
            (159, 2),
            (2504, 1),
            (2101, 1),
            (2512, 200),
            (117, 4),
            (6948, 1),
            (271668, 1),
            (271666, 1),
            (271665, 1),
            (2092, 1),
        ],
        4 => vec![
            (4540, 4),
            (2947, 1),
            (6948, 1),
            (271673, 1),
            (271674, 1),
            (271675, 1),
            (2092, 1),
        ],
        7 => vec![
            (117, 4),
            (159, 2),
            (6948, 1),
            (271661, 1),
            (271662, 1),
            (271663, 1),
            (280400, 1),
        ],
        8 => vec![
            (35, 1),
            (117, 4),
            (159, 2),
            (6948, 1),
            (271659, 1),
            (271658, 1),
            (271655, 1),
        ],
        11 => vec![
            (6948, 1),
            (3661, 1),
            (159, 2),
            (4536, 4),
            (271668, 1),
            (271666, 1),
            (271665, 1),
        ],
        _ => panic!("not an authored Skyborne class"),
    }
}

fn authored_pairs() -> impl Iterator<Item = (u8, u8)> {
    [
        (95, 1),
        (95, 3),
        (95, 4),
        (95, 8),
        (95, 11),
        (96, 1),
        (96, 3),
        (96, 4),
        (96, 7),
        (96, 11),
    ]
    .into_iter()
}

#[test]
fn forever_memberships_and_counts_cover_all_ten_pairs_without_retail_leakage() {
    let (_directory, path) = fixture_path();
    let kit = load_kit(&path);
    let actual: Vec<_> = authored_pairs()
        .map(|(race, class)| {
            (
                race,
                class,
                kit.create_info.get(race, class).unwrap().items.clone(),
            )
        })
        .collect();
    let expected: Vec<_> = authored_pairs()
        .map(|(race, class)| (race, class, expected_items(class)))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(
        kit.create_info.get(1, 8).unwrap().items,
        vec![
            (35, 1),
            (6948, 1),
            (187762, 1),
            (187761, 1),
            (187760, 1),
            (187759, 1),
            (187758, 1),
            (187757, 1)
        ]
    );
}

#[test]
fn forever_creation_equips_concrete_gear_and_persists_all_authored_items_on_reopen() {
    use shared::item_data::ItemDefinitionSource::Forever70205;
    let (directory, path) = fixture_path();
    let kit = load_kit(&path);
    for (race, class) in authored_pairs() {
        let player_path = directory.path().join(format!("{race}-{class}.redb"));
        let db = redb::Database::create(&player_path).unwrap();
        crate::persistence::create_tables(&db);
        let (id, created) = create_character_with_appearance(
            &db,
            1,
            "Skyborne",
            race,
            class,
            CharacterAppearance::default(),
            &kit.kit(),
        )
        .unwrap();
        assert_eq!(created.level, 1);
        drop(db);
        let db = redb::Database::open(&player_path).unwrap();
        crate::persistence::create_tables(&db);
        let record = load_inventory_record(&db, id).unwrap();
        assert_eq!(record.character, created);
        for entry in &record.character.equipment_appearance.entries {
            assert_eq!(entry.definition_source, Some(Forever70205));
            let owned = record
                .equipped
                .iter()
                .find(|owned| owned.slot == entry.slot)
                .unwrap();
            assert_eq!(entry.item_id, Some(owned.item.item_id));
            assert_eq!(
                record.definition_sources[&owned.item.item_guid],
                Forever70205
            );
        }
        assert_eq!(
            (record.character.race, record.character.class),
            (race, class)
        );
        let mut actual: Vec<_> = record
            .inventory
            .items
            .iter()
            .chain(record.equipped.iter().map(|entry| &entry.item))
            .chain(record.bag_slots.iter().map(|entry| &entry.item))
            .map(|item| (item.item_id, item.stack_count))
            .collect();
        actual.sort();
        let mut expected = expected_items(class);
        expected.sort();
        assert_eq!(
            actual, expected,
            "race {race} class {class} retains every genuine kit item/count"
        );
        assert_eq!(
            record.equipped.len(),
            if class == 1 { 5 } else { 4 },
            "concrete gear count for {race}/{class}"
        );
        if class == 3 {
            assert_eq!(record.bag_slots.len(), 1);
            let quiver = &record.bag_slots[0];
            assert_eq!(
                (quiver.bag, quiver.item.item_id, quiver.item.stack_count),
                (1, 2101, 1)
            );
            assert_eq!(
                record.definition_sources[&quiver.item.item_guid],
                Forever70205
            );
            let template = kit
                .templates
                .get(Forever70205, quiver.item.item_id)
                .unwrap();
            assert_eq!(
                (
                    template.inventory_type,
                    template.container_slots,
                    template.scaling.class_id,
                    template.bag_family
                ),
                (18, 6, 11, 1)
            );
        } else {
            assert!(record.bag_slots.is_empty());
        }
        assert_eq!(
            record.character.equipment_appearance.entries.len(),
            record.equipped.len()
        );
        let equipment_ids: Vec<_> = record
            .equipped
            .iter()
            .map(|entry| entry.item.item_id)
            .collect();
        let clothes: &[u32] = match class {
            1 => &[271671, 271672, 271669],
            3 | 11 => &[271668, 271666, 271665],
            4 => &[271673, 271674, 271675],
            7 => &[271661, 271662, 271663],
            8 => &[271659, 271658, 271655],
            _ => unreachable!(),
        };
        for clothing in clothes {
            assert!(equipment_ids.contains(clothing));
        }
        crate::persistence::save_character(&db, id, &record.character);
        drop(db);
        let db = redb::Database::open(&player_path).unwrap();
        assert_eq!(
            crate::persistence::load_character(&db, id).unwrap(),
            created
        );
    }
}

#[test]
fn shared_retail_masks_and_overrides_do_not_leak_into_forever_kits() {
    let (_directory, path) = fixture_path();
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE character_loadout SET RaceMasks_0=1 WHERE ID=1486",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tdb_playercreateinfo_item (race,class,itemid,amount) VALUES (0,0,35,1)",
        [],
    )
    .unwrap();
    let infos = load_player_create_infos(&conn).unwrap();
    assert_eq!(infos.get(95, 1).unwrap().items, expected_items(1));
    assert_eq!(infos.get(96, 7).unwrap().items, expected_items(7));
    assert_eq!(
        infos.get(1, 1).unwrap().items,
        vec![
            (58231, 1),
            (39, 1),
            (40, 1),
            (2362, 1),
            (25, 1),
            (6948, 1),
            (35, 1)
        ]
    );
    assert_eq!(
        conn.query_row(
            "SELECT RaceMasks_0,RaceMasks_1 FROM character_loadout WHERE ID=1486",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        )
        .unwrap(),
        (1, 1)
    );
}

#[test]
fn missing_forever_content_fails_instead_of_silently_skipping_required_items() {
    let (_directory, path) = fixture_path();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM skyborne_content_item WHERE ID=271655", [])
        .unwrap();
    let error = load_player_create_infos(&conn)
        .expect_err("required shirt must not disappear from the kit");
    assert!(error.to_string().contains("271655"), "{error}");
}
