//! TC a352b1fa374364 once-effect destinations and native374362→374360 summons.
use super::*;

const SPROUT: u32 = 190509;
const ORIGINS: [(f32, f32, f32); 7] = [
    (-5129.39, 1253.30, 555.58),
    (-5101.68, 1253.71, 555.90),
    (-5114.70, 1230.28, 555.89),
    (-5141.62, 1230.33, 555.83),
    (-5155.62, 1253.60, 555.87),
    (-5141.42, 1276.70, 555.89),
    (-5114.78, 1277.42, 555.87),
];

fn prepare_sprout_templates(h: &mut Harness) {
    let (_directory, path) = prepare_encounter_database("leymor");
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = h.app.world().resource::<WorldData>().expected_stats.clone();
    let mut templates = crate::world_data::load_templates(&connection, &expected);
    templates.retain(|entry, _| *entry == SPROUT);
    assert_eq!(
        templates.len(),
        1,
        "authentic runtime sprout template prerequisite"
    );
    crate::world_data::load_template_models(&connection, &mut templates);
    let difficulties =
        crate::world_data::load_difficulty_templates(&connection, &expected, &templates);
    let mut data = h.app.world_mut().resource_mut::<WorldData>();
    data.templates.extend(templates);
    data.difficulty_templates.extend(difficulties);
}

fn install_azure_ground(h: &mut Harness, map: InMap) {
    let wdt = h.app.world().resource::<MapStore>().maps[&2515].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(2515, wdt)]),
    ));
    for (x, y, z) in ORIGINS {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
        let probe = h
            .app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .ground_in(map, Vec3::new(x, y, z));
        assert!(
            matches!(probe, crate::ground::GroundProbe::Supported(_)),
            "authentic origin floor prerequisite: {probe:?}"
        );
    }
}

#[test]
#[ignore = "RED: once-effect Ley-Line Sprouts destination children not implemented"]
fn azure_leymor_sprouts_parent_summons_two_native_children_per_authored_origin() {
    let (mut h, leymor) = super::azure_combat::wake_leymor();
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    prepare_sprout_templates(&mut h);
    install_azure_ground(&mut h, map);
    assert!(
        h.creatures_of(SPROUT).is_empty(),
        "producer inserts no placements"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), leymor, 374364, Some(leymor), true)
        .expect("native once-effect parent cast");
    // Native missile18yd/s, minimum1.5s; authored groups are within40yd of Leymor.
    h.advance(6.0);
    let children = h.creatures_of(SPROUT);
    assert_eq!(
        children.len(),
        14,
        "seven source groups, two actual native summons each"
    );
    for child in &children {
        assert_eq!(h.app.world().get::<InMap>(*child), Some(&map));
        assert_eq!(
            h.app
                .world()
                .get::<crate::spell_cast::summon::SummonedCreature>(*child)
                .expect("native summon owner")
                .owner,
            leymor
        );
    }
    let positions: Vec<_> = children
        .into_iter()
        .map(|child| h.wow_position(child))
        .collect();
    for (x, y, _) in ORIGINS {
        let origin = Vec2::new(x, y);
        let count = positions
            .iter()
            .filter(|position| Vec2::new(position.x, position.y).distance(origin) <= 7.001)
            .count();
        assert_eq!(count, 2, "native target86/radius37 group at {origin:?}");
    }
}
