//! Simulator view of transport registrations already used by server interaction systems.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Action {
    SpellClick,
    Proximity,
}

#[derive(Clone, Debug)]
pub struct Registration {
    pub entry: u32,
    pub source: Vec3,
    pub destination: Vec3,
    pub distance: f32,
    pub action: Action,
}

/// Read registrations from the same constants and taxi network as live gameplay.
pub fn read_registrations(
    map: u32,
    db: &rusqlite::Connection,
) -> Result<Vec<Registration>, String> {
    let mut registrations = Vec::new();
    if map == crate::taxi::RUBY_FERRY_MAP {
        let network =
            crate::taxi::load_taxi_network(db).map_err(|error| format!("taxi network: {error}"))?;
        let points = network
            .scripted_path(crate::taxi::RUBY_FERRY_PATH)
            .ok_or_else(|| {
                format!(
                    "registered taxi path{} unavailable",
                    crate::taxi::RUBY_FERRY_PATH
                )
            })?;
        let destination = *points.last().ok_or("registered taxi path has no points")?;
        let distance = points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum();
        registrations.push(Registration {
            entry: crate::taxi::RUBY_FERRY_ENTRY,
            source: crate::taxi::RUBY_FERRY_SOURCE,
            destination,
            distance,
            action: Action::SpellClick,
        });
    }
    if map == crate::scripted_ai::academy_transport::MAP {
        registrations.extend(crate::scripted_ai::academy_transport::PADS.iter().map(
            |(entry, source, destination)| Registration {
                entry: *entry,
                source: *source,
                destination: *destination,
                distance: source.distance(*destination),
                action: Action::Proximity,
            },
        ));
    }
    Ok(registrations)
}

pub fn install_taxi_system(app: &mut App, db: &rusqlite::Connection) -> Result<(), String> {
    let network =
        crate::taxi::load_taxi_network(db).map_err(|error| format!("taxi network: {error}"))?;
    app.insert_resource(network);
    app.add_systems(FixedUpdate, crate::taxi::fly_taxis);
    Ok(())
}

pub fn interaction_distance() -> f32 {
    crate::npc_interaction::INTERACTION_DISTANCE
}

pub fn in_progress(world: &World, unit: Entity) -> bool {
    world.get::<crate::taxi::TaxiFlight>(unit).is_some()
        || world
            .get::<crate::scripted_ai::academy_transport::AcademyPadFlight>(unit)
            .is_some()
}

pub fn activate(
    world: &mut World,
    owner: Entity,
    npc: Entity,
    action: Action,
) -> Result<(), String> {
    match action {
        Action::SpellClick => {
            let entry = world
                .get::<shared::components::Npc>(npc)
                .ok_or("transport source is not an NPC")?
                .template_id;
            crate::npc_interaction::spellclick::cast_spellclick(world, owner, npc, entry);
            if world.get::<crate::taxi::TaxiFlight>(owner).is_none() {
                return Err(format!(
                    "transport spellclick npc{entry} did not start taxi flight"
                ));
            }
        }
        Action::Proximity => {}
    }
    Ok(())
}

#[test]
fn ruby_registration_uses_native_taxi_points_and_academy_uses_pad_registry() {
    let path = crate::test_data::world_db_path();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let ruby = read_registrations(2521, &db).unwrap();
    assert_eq!(ruby.len(), 1);
    assert_eq!(ruby[0].entry, 194667);
    assert_eq!(ruby[0].action, Action::SpellClick);
    assert!(ruby[0].destination.y > ruby[0].source.y);
    let academy = read_registrations(2526, &db).unwrap();
    assert_eq!(academy.len(), 5);
    assert!(academy.iter().all(|row| row.action == Action::Proximity));
}
