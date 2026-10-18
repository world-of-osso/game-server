use super::targets::{Dest, ground_destination};
use crate::{ground::GroundMap, maps::InMap};
use bevy::prelude::*;
use shared::components::{Position, Rotation};
use shared::protocol::{GameObjectCollision, GameObjectInfo, GameObjectVisible};
use std::{collections::HashMap, path::PathBuf};

/// TC `TARGET_DEST_CASTER_GROUND` uses `Map::GetHeight(phaseShift, ...)`, which includes
/// the enabled dynamic GO floor; the closed Cage trapdoor is that floor until it opens.
#[test]
fn arena_live_caster_ground_destination_lands_on_enabled_trapdoor() {
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        data.join("diagnostics/arena-coverage-2026-10-12/canonical-proof-data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt: u32 = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2759", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut app = App::new();
    app.insert_resource(GroundMap::new(&data, HashMap::from([(2759, wdt)])));
    app.insert_resource(crate::pvp::arena::Catalog::load(&db).unwrap());
    app.add_plugins(crate::game_objects::collision::CollisionIndexPlugin);
    app.add_systems(
        FixedUpdate,
        crate::game_objects::collision::sync_live_objects,
    );
    let map = InMap {
        map_id: 2759,
        instance_id: 786,
    };
    let caster = app.world_mut().spawn(map).id();
    let door = app
        .world_mut()
        .spawn((
            map,
            Position {
                x: 443.59,
                y: -24.8806,
                z: -321.262,
            },
            Rotation {
                x: 0.0,
                y: 1.5708 + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            GameObjectInfo {
                entry: 505683,
                go_type: 35,
                display_id: 99831,
                name: "Trapdoor".into(),
                scale: 1.0,
            },
            GameObjectCollision(true),
            GameObjectVisible(true),
        ))
        .id();
    app.world_mut().run_schedule(FixedUpdate);
    let effect = crate::spell_info::SpellEffect {
        implicit_targets: [62, 0],
        ..Default::default()
    };
    let requested = Dest {
        position: Vec3::new(443.2981, -20.0, -323.9937),
        orientation: 0.0,
    };
    let closed = ground_destination(
        app.world_mut(),
        caster,
        &crate::spell_info::SpellInfo::default(),
        &effect,
        Some(requested),
    )
    .unwrap()
    .unwrap()
    .position
    .y;
    assert!(
        (closed + 24.38845).abs() < 0.05,
        "closed trapdoor must be the caster-ground floor, got {closed}"
    );
    app.world_mut()
        .entity_mut(door)
        .insert(GameObjectCollision(false));
    app.world_mut().run_schedule(FixedUpdate);
    let opened = ground_destination(
        app.world_mut(),
        caster,
        &crate::spell_info::SpellInfo::default(),
        &effect,
        Some(requested),
    )
    .unwrap()
    .unwrap()
    .position
    .y;
    assert!(
        opened < -30.0,
        "opened trapdoor must leave only static ground, got {opened}"
    );
}
