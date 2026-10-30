//! Install production follower flight in a headless simulator.
use bevy::prelude::*;
use std::path::Path;

pub fn install(app: &mut App, db: &rusqlite::Connection, nav_root: &Path) -> Result<(), String> {
    if !app.world().contains_resource::<crate::ground::GroundMap>() {
        app.insert_resource(super::transport::open_ground(db)?);
    }
    if !app
        .world()
        .contains_resource::<crate::navigation::Navigation>()
    {
        app.insert_resource(crate::navigation::Navigation::new(nav_root));
    }
    app.insert_resource(
        crate::area_table::load_area_table(db).map_err(|error| format!("flight areas: {error}"))?,
    );
    Ok(())
}

/// Advance the same flight-follow and landing system used by CreatureAiPlugin.
pub fn update(world: &mut World) {
    crate::creature_ai::flight_follow::update(world);
}
