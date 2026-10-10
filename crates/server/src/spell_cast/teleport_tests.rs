use super::test_harness::*;
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{MovementControl, Position, Rotation};

const BLINK: u32 = 1953;

#[test]
fn spell_teleport_new_character_bind_is_persisted_at_creation_not_login() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new-character.redb");
    let db = redb::Database::create(&path).unwrap();
    crate::persistence::create_tables(&db);
    let (id, mut character) =
        crate::persistence_character::create_character(&db, 1, "Bound", 1, 8).unwrap();
    let bind =
        super::homebind::load_bind(&db, id).expect("new character has its actual creation bind");
    assert_eq!(
        bind.0.position,
        Vec3::new(character.x, character.y, character.z)
    );
    character.x += 100.0;
    crate::persistence_character::save_character(&db, id, &character);
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(super::homebind::load_bind(&db, id).unwrap().0, bind.0);
    assert!(
        super::homebind::load_bind(&db, id + 1).is_none(),
        "missing bind must not be fabricated from race or logout position"
    );
}

#[test]
fn spell_teleport_fixed_destination_consumes_authored_fixture() {
    // Routing/loader fixture, not a claim about Retail Moonglade's actual coordinates.
    // Spell ID is matrix18960; the authored point is the existing map0 race start.
    let mut h = harness(Vec::new(), &[18960], 11).unwrap();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(18960)
        .unwrap();
    let index = spell
        .effects
        .iter()
        .find(|e| e.effect == 252)
        .unwrap()
        .index;
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!(
        "../../../../scripts/spell_target_position_schema.sql"
    ))
    .unwrap();
    conn.execute("INSERT INTO spell_target_position (ID,EffectIndex,MapID,PositionX,PositionY,PositionZ,Orientation) VALUES (?1,?2,0,-8949,-132,83,1.2)",rusqlite::params![18960,index]).unwrap();
    h.app
        .insert_resource(super::teleport_positions::load_positions(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::CombatStatus(false));
    h.cast(18960, None);
    h.advance(250);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        crate::maps::WorldLocation {
            map_id: 0,
            position: Vec3::new(-8949.0, -132.0, 83.0),
            orientation: 1.2
        }
        .bevy_position()
    );
}

#[test]
fn spell_teleport_fixed_destination_without_authored_row_fails_explicitly() {
    let mut h = harness(Vec::new(), &[18960], 11).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::CombatStatus(false));
    h.app
        .insert_resource(super::teleport_positions::SpellTargetPositions::default());
    h.cast(18960, None);
    h.advance(4);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(
        h.inbox.failed[0].detail.as_deref(),
        Some("missing SpellTargetPosition row")
    );
}

#[test]
fn spell_teleport_ground_leap_rejects_out_of_range_and_nonfinite_destinations() {
    use lightyear::prelude::*;
    for point in [[10000.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0]] {
        let mut h = harness(Vec::new(), &[189110], 12).unwrap();
        let request = shared::protocol::SpellCastIntent {
            destination: Some(point),
            spell_id: Some(189110),
            spell: String::new(),
            target_entity: None,
            witness: None,
        };
        h.app
            .world_mut()
            .get_mut::<MessageSender<shared::protocol::SpellCastIntent>>(h.connection)
            .unwrap()
            .send::<shared::protocol::CombatChannel>(request);
        h.advance(4);
        assert_eq!(h.inbox.failed.len(), 1);
        assert_eq!(
            *h.app.world().get::<Position>(h.caster).unwrap(),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        );
    }
}

#[test]
fn spell_teleport_matrix_infernal_strike_uses_requested_ground_destination() {
    use lightyear::prelude::*;
    let mut h = harness(Vec::new(), &[189110], 12).unwrap();
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 2.0,
        ));
    let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data/los");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(dir).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 10.0,
            y: 2.0,
            z: -10.0,
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    let request: shared::protocol::SpellCastIntent = serde_json::from_value(serde_json::json!({
        "spell_id":189110,"spell":"","target_entity":null,"witness":null,"destination":[30.0,2.0,-10.0]
    })).unwrap();
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::SpellCastIntent>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(request);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        Position {
            x: 30.0,
            y: 2.0,
            z: -10.0
        }
    );
}

#[test]
fn spell_teleport_matrix_harpoon_and_feral_lunge_move_caster_not_target() {
    for id in [186260, 196884] {
        // Harpoon190925 trigger and matrix Feral Lunge.
        let mut h = harness(Vec::new(), &[id], 7).unwrap();
        h.app
            .insert_resource(crate::terrain_height::test_helpers::flat_ground(
                33.0, 33.0, 2.0,
            ));
        let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data/los");
        h.app
            .insert_resource(crate::line_of_sight::LineOfSight::load(dir).unwrap());
        let start = Position {
            x: 10.0,
            y: 2.0,
            z: -10.0,
        };
        let destination = Position {
            x: 30.0,
            y: 2.0,
            z: -10.0,
        };
        h.app.world_mut().entity_mut(h.caster).insert((
            start,
            Rotation {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ));
        h.app.world_mut().entity_mut(h.npc).insert(destination);
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .unwrap()
            .clone();
        let plan =
            super::plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), true).unwrap();
        super::resolve_effects(h.app.world_mut(), h.caster, &spell, &plan, 0);
        assert_eq!(
            *h.app.world().get::<Position>(h.caster).unwrap(),
            destination,
            "spell{id}"
        );
        assert_eq!(*h.app.world().get::<Position>(h.npc).unwrap(), destination);
    }
}

#[test]
fn spell_teleport_blink_lands_twenty_yards_forward_on_ground() {
    let mut h = harness(Vec::new(), &[BLINK], 8).unwrap();
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 2.0,
        ));
    let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data/los");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(dir).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 10.0,
            y: 2.0,
            z: -10.0,
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        MovementControl::default(),
    ));
    h.cast(BLINK, None);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let pos = h.app.world().get::<Position>(h.caster).unwrap();
    assert!((pos.x - 30.0).abs() < 0.01, "{pos:?}");
    assert_eq!(pos.y, 2.0);
    assert_eq!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .epoch,
        1
    );
}

#[test]
fn spell_teleport_blink_stops_before_real_map_zero_abbey_wall() {
    let mut h = harness(Vec::new(), &[BLINK], 8).unwrap();
    let dir = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&dir.join("los")).unwrap());
    h.app.insert_resource(crate::ground::GroundMap::new(
        dir,
        std::collections::HashMap::from([(0, 775971)]),
    ));
    // Existing real-geometry LOS fixture: the doorway line shifted four yards into its wall.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-8905.575, -144.91, 81.96);
    let start = Vec3::new(x, y, z);
    let direction = Vec3::new(12.7, 0.0, 27.2).normalize();
    let yaw = direction.x.atan2(direction.z);
    let los = h
        .app
        .world()
        .resource::<crate::line_of_sight::LineOfSight>();
    assert!(!los.clear(
        InMap::continent(0),
        start + Vec3::Y,
        start + direction * 20.0 + Vec3::Y
    ));
    h.app.world_mut().entity_mut(h.caster).insert((
        Position { x, y, z },
        Rotation {
            x: 0.0,
            y: yaw,
            z: 0.0,
        },
        MovementControl::default(),
    ));
    h.cast(BLINK, None);
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let p = h.app.world().get::<Position>(h.caster).unwrap();
    let end = Vec3::new(p.x, p.y, p.z);
    let distance = Vec2::new(end.x - start.x, end.z - start.z).length();
    assert!(distance > 0.5 && distance < 19.0, "moved {distance}: {p:?}");
    assert!(
        h.app
            .world()
            .resource::<crate::line_of_sight::LineOfSight>()
            .clear(InMap::continent(0), start + Vec3::Y, end + Vec3::Y)
    );
}

#[test]
fn spell_teleport_hearthstone_returns_to_persisted_bind() {
    let mut h = harness(Vec::new(), &[8690], 8).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bind.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        super::homebind::save_bind(
            &db,
            7,
            crate::maps::WorldLocation {
                map_id: 0,
                position: Vec3::new(-8949.0, -132.0, 83.0),
                orientation: 1.2,
            },
        );
    }
    let db = redb::Database::open(&path).unwrap();
    let bind = super::homebind::load_bind(&db, 7).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((shared::components::CombatStatus(false), bind));
    h.cast(8690, None);
    h.advance(250);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let position = h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(*position, bind.0.bevy_position());
    assert!(
        (h.app.world().get::<Rotation>(h.caster).unwrap().y - (1.2 + std::f32::consts::FRAC_PI_2))
            .abs()
            < 0.01
    );
}
