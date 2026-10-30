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
    app.insert_resource(
        crate::auras::mount::load_mount_store(db)
            .map_err(|error| format!("flight mount catalog: {error}"))?,
    );
    crate::navigation::install_publication(app);
    Ok(())
}

/// Explicit DAWNBREAKER_LEADER_FLIGHT_FIXTURE: Golden Gryphon and Expert Riding.
pub fn configure_leader(world: &mut World, owner: Entity) -> Result<(), String> {
    use crate::auras::mount::{FlightStyle, SKILL_RIDING};
    use crate::profession::PlayerProfessions;
    use shared::profession::ProfessionSkillLine;

    if world.get::<shared::components::Player>(owner).is_none() {
        return Err("flight fixture leader is not a player".into());
    }
    let mut professions = world
        .entity_mut(owner)
        .take::<PlayerProfessions>()
        .unwrap_or_default();
    professions
        .lines
        .retain(|skill| skill.skill_line != SKILL_RIDING);
    professions.lines.push(ProfessionSkillLine {
        skill_line: SKILL_RIDING,
        step: 3,
        rank: 225,
        max_rank: 225,
    });
    world.entity_mut(owner).insert(professions);
    let mut known = world
        .entity_mut(owner)
        .take::<crate::class_progression::KnownSpells>()
        .unwrap_or_default();
    known.grant_fixture_spell(32235);
    world.entity_mut(owner).insert(known);
    if world
        .get::<shared::components::PlayerMotion>(owner)
        .is_none()
    {
        world
            .entity_mut(owner)
            .insert(shared::components::PlayerMotion::default());
    }
    crate::auras::mount::set_flight_style(world, owner, FlightStyle::Steady);
    if !crate::auras::has_aura(world, owner, crate::auras::mount::FLIGHT_STYLE_STEADY) {
        return Err("native steady flight style did not apply".into());
    }
    Ok(())
}

/// Submit an ordinary mount cast; native mount and area rules grant CAN_FLY.
pub fn request_mount(world: &mut World, owner: Entity) -> Result<(), String> {
    if !world
        .get::<crate::class_progression::KnownSpells>(owner)
        .is_some_and(|spells| spells.contains(32235))
    {
        return Err("leader does not know the flight fixture mount".into());
    }
    crate::spell_cast::creature_cast(world, owner, 32235, None, false)
        .map_err(|error| format!("native flight mount cast: {error:?}"))
}

/// Set the player input's flying state only after native flight permission is granted.
pub fn enter_flight(world: &mut World, owner: Entity) -> Result<(), String> {
    let position = *world
        .get::<shared::components::Position>(owner)
        .ok_or("flight leader lacks position")?;
    let map = *world
        .get::<crate::maps::InMap>(owner)
        .ok_or("flight leader lacks map")?;
    let area = world
        .resource_mut::<crate::ground::GroundMap>()
        .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))
        .ok_or("flight leader area is unavailable")?;
    if !world
        .resource::<crate::area_table::AreaTable>()
        .permits_flying(area, map.map_id)
    {
        return Err(format!("flight is not permitted in area {area}"));
    }
    let mut motion = world
        .get_mut::<shared::components::PlayerMotion>(owner)
        .ok_or("flight leader lacks motion")?;
    if !motion.contains(shared::components::PlayerMotion::CAN_FLY) {
        return Err("native mount did not grant flight permission".into());
    }
    motion.0 |= shared::components::PlayerMotion::FLYING;
    Ok(())
}

pub fn land(world: &mut World, owner: Entity) -> Result<(), String> {
    let mut motion = world
        .get_mut::<shared::components::PlayerMotion>(owner)
        .ok_or("flight leader lacks motion")?;
    motion.0 &= !shared::components::PlayerMotion::FLYING;
    crate::auras::mount::cancel_mount_aura(world, owner);
    Ok(())
}

/// Advance the same flight-follow and landing system used by CreatureAiPlugin.
pub fn update(world: &mut World) {
    crate::creature_ai::flight_follow::update(world);
}
