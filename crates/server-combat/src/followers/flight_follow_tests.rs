//! Follower flight uses production movement and a real landing navmesh.
use super::navigation_tests::stockade_fixture;
use crate::ground::GroundMap;
use bevy::prelude::*;
use shared::components::{Mounted, Npc, PlayerMotion, Position, UnitPowers};

fn areas(flags: u32) -> crate::area_table::AreaTable {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContinentID INTEGER, Flags_0 INTEGER, FactionGroupMask INTEGER, Flags_1 INTEGER, AreaName_lang TEXT, MountFlags INTEGER);").unwrap();
    db.execute(
        "INSERT INTO area_table VALUES (9001,0,34,0,0,0,'Flight fixture',?1)",
        [flags],
    )
    .unwrap();
    crate::area_table::load_area_table(&db).unwrap()
}

fn spawn_druid(h: &mut crate::spell_cast::test_harness::Harness, position: Position) -> Entity {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profile = super::catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.entry == 209072)
        .unwrap();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[209072]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let unit = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert((
        Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        UnitPowers::default(),
        position,
        crate::maps::InMap::continent(34),
    ));
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    unit
}

#[test]
fn flying_follower_requires_area_permission_and_lands_on_leaders_navmesh() {
    let from = Position {
        x: 150.557,
        y: -34.85624,
        z: -45.8164,
    };
    let (mut h, follower, _) = stockade_fixture(from);
    let owner = h.caster;
    let druid = spawn_druid(&mut h, from);
    let landing = Position {
        x: 103.0,
        y: -34.85623,
        z: -76.0,
    };
    let in_air = Position {
        y: landing.y + 20.0,
        ..landing
    };
    h.app.insert_resource(GroundMap::with_area_samples(&[(
        34,
        Vec3::new(in_air.x, in_air.y, in_air.z),
        9001,
    )]));
    h.app.insert_resource(areas(0));
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(owner)
        .unwrap()
        .class = 11;
    crate::spell_cast::creature_cast(h.app.world_mut(), owner, 33943, Some(owner), true)
        .expect("leader enters native Flight Form");
    assert!(crate::auras::has_aura(h.app.world(), owner, 33943));
    h.app.world_mut().entity_mut(owner).insert((
        in_air,
        PlayerMotion(PlayerMotion::CAN_FLY | PlayerMotion::FLYING),
    ));
    assert!(
        h.app.world().get::<Mounted>(owner).is_none(),
        "leader is in an unmounted flight form"
    );
    h.advance(5);
    assert!(h.app.world().get::<Mounted>(follower).is_none());
    assert!(!crate::auras::has_aura(h.app.world(), druid, 33943));
    h.app.insert_resource(areas(2));
    for _ in 0..500 {
        h.advance(1);
    }
    let airborne = *h.app.world().get::<Position>(follower).unwrap();
    assert!(
        (airborne.y - in_air.y).abs() < 1.0,
        "follower did not follow in three dimensions: {airborne:?}"
    );
    assert_eq!(
        h.app
            .world()
            .get::<Mounted>(follower)
            .unwrap()
            .mount_display_id,
        17697
    );
    assert!(h.app.world().get::<Mounted>(druid).is_none());
    assert!(
        crate::auras::has_aura(h.app.world(), druid, 33943),
        "druid uses its own Flight Form"
    );
    let druid_air = *h.app.world().get::<Position>(druid).unwrap();
    assert!((druid_air.y - in_air.y).abs() < 1.0);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((landing, PlayerMotion::default()));
    for _ in 0..500 {
        h.advance(1);
        if h.app.world().get::<Mounted>(follower).is_none()
            && !crate::auras::has_aura(h.app.world(), druid, 33943)
        {
            break;
        }
        h.app
            .world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
    }
    let landed = *h.app.world().get::<Position>(follower).unwrap();
    assert!(
        h.app.world().get::<Mounted>(follower).is_none(),
        "follower did not complete navmesh landing: {landed:?}"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), druid, 33943),
        "druid leaves Flight Form after landing"
    );
    assert!((landed.y - landing.y).abs() < navigation::AGENT_CLIMB);
    assert!((landed.x - landing.x).hypot(landed.z - landing.z) < 1.0);
    let nav_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh/34");
    let mut nav = navigation::NavigationMap::open(&nav_dir).unwrap();
    let at = Vec3::new(landed.x, landed.y, landed.z);
    let leader = Vec3::new(landing.x, landing.y, landing.z);
    let floor = nav
        .floor_polygon(at)
        .unwrap()
        .expect("follower has a navmesh floor");
    assert!((floor.y - at.y).abs() < 0.05);
    assert!(
        nav.path(leader, at).unwrap().complete,
        "landing must connect to the leader"
    );
}
