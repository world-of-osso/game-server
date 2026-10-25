use super::*;

fn spawn_terrain_connection(app: &mut App, player: Entity) -> Entity {
    app.insert_resource(crate::maps::MapStore::continents());
    app.world_mut()
        .spawn((PlayerLink(player), SentTiles::default()))
        .id()
}

fn run_terrain_streaming(app: &mut App) {
    app.world_mut()
        .run_system_once(stream_terrain_tiles)
        .expect("terrain streaming should run");
}

fn sent_tiles_tick(app: &App, connection: Entity) -> bevy::ecs::change_detection::Tick {
    app.world()
        .entity(connection)
        .get_ref::<SentTiles>()
        .expect("connection should have SentTiles")
        .last_changed()
}

#[test]
fn same_tile_movement_does_not_dirty_sent_tiles() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 100.0, 0.0, -100.0);
    let connection = spawn_terrain_connection(&mut app, player);

    run_terrain_streaming(&mut app);
    let initial_tiles = app
        .world()
        .entity(connection)
        .get::<SentTiles>()
        .unwrap()
        .tiles
        .clone();
    let initial_tick = sent_tiles_tick(&app, connection);
    assert_eq!(initial_tiles.len(), 9);

    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 120.0;
    app.update();
    run_terrain_streaming(&mut app);

    let sent = app.world().entity(connection).get::<SentTiles>().unwrap();
    assert_eq!(sent.tiles, initial_tiles);
    assert_eq!(
        sent_tiles_tick(&app, connection),
        initial_tick,
        "same-tile movement must not re-evaluate and dirty SentTiles"
    );
}

#[test]
fn tile_boundary_crossing_updates_sent_tiles() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 100.0, 0.0, -100.0);
    let connection = spawn_terrain_connection(&mut app, player);

    run_terrain_streaming(&mut app);
    let initial_tick = sent_tiles_tick(&app, connection);
    let initial_len = app
        .world()
        .entity(connection)
        .get::<SentTiles>()
        .unwrap()
        .tiles
        .len();

    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 600.0;
    app.update();
    run_terrain_streaming(&mut app);

    let sent = app.world().entity(connection).get::<SentTiles>().unwrap();
    assert!(sent.tiles.len() > initial_len);
    assert_ne!(sent_tiles_tick(&app, connection), initial_tick);
    let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(600.0, 0.0, -100.0);
    assert!(sent.tiles.contains(&wow_to_tile(wow_x, wow_y)));
}

#[test]
fn fresh_connection_forces_initial_tile_evaluation() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 100.0, 0.0, -100.0);
    let first_connection = spawn_terrain_connection(&mut app, player);
    run_terrain_streaming(&mut app);
    assert_eq!(
        app.world()
            .entity(first_connection)
            .get::<SentTiles>()
            .unwrap()
            .tiles
            .len(),
        9
    );

    app.world_mut().despawn(first_connection);
    let second_connection = spawn_terrain_connection(&mut app, player);
    run_terrain_streaming(&mut app);

    assert_eq!(
        app.world()
            .entity(second_connection)
            .get::<SentTiles>()
            .unwrap()
            .tiles
            .len(),
        9
    );
}
