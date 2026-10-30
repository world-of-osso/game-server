//! Native interaction objectives exposed to the headless route driver.
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{Npc, Position};

#[derive(Debug)]
pub enum InteractionStep {
    Approach { target: Entity },
    Interact { target: Entity },
    Wait,
}

pub fn install(app: &mut App, db: &rusqlite::Connection) -> Result<(), String> {
    app.insert_resource(
        crate::npc_interaction::spellclick::load_spellclick_catalog(db)
            .map_err(|error| format!("objective spellclick catalog: {error}"))?,
    );
    app.add_systems(
        FixedUpdate,
        crate::npc_interaction::nalorakk_offerings::update_offerings,
    );
    Ok(())
}

pub fn next(
    world: &mut World,
    owner: Entity,
    objective: Entity,
) -> Result<Option<InteractionStep>, String> {
    let map = *world
        .get::<InMap>(owner)
        .ok_or("interaction owner lacks map")?;
    let Some(npc) = world.get::<Npc>(objective) else {
        return Ok(None);
    };
    if map.map_id != 2825
        || npc.template_id != 241812
        || crate::npc_interaction::nalorakk_offerings::all_avatars_fed(world, map)
    {
        return Ok(None);
    }
    let origin = *world
        .get::<Position>(owner)
        .ok_or("interaction owner lacks position")?;
    let point = Vec3::new(origin.x, origin.y, origin.z);
    if world
        .get::<crate::npc_interaction::nalorakk_offerings::PendingOffering>(owner)
        .is_some()
    {
        return Ok(Some(InteractionStep::Wait));
    }
    let carrying = world
        .get::<crate::npc_interaction::nalorakk_offerings::CarryingOffering>(owner)
        .is_some();
    let mut candidates = world
        .query::<(
            Entity,
            &Npc,
            &Position,
            &InMap,
            Option<&crate::npc_interaction::nalorakk_offerings::AvatarFed>,
        )>()
        .iter(world)
        .filter(|(_, npc, _, other, fed)| {
            **other == map
                && if carrying {
                    npc.template_id == 241805 && fed.is_none()
                } else {
                    crate::npc_interaction::nalorakk_offerings::is_offering_source(npc.template_id)
                }
        })
        .map(|(entity, _, at, _, _)| (entity, point.distance(Vec3::new(at.x, at.y, at.z))))
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.to_bits().cmp(&b.0.to_bits())));
    let (target, distance) = candidates
        .first()
        .copied()
        .ok_or("OBJECTIVE_INTERACTION_SOURCE_MISSING")?;
    let radius = if carrying {
        crate::npc_interaction::nalorakk_offerings::DELIVERY_RADIUS
    } else {
        1.0
    };
    if distance > radius {
        return Ok(Some(InteractionStep::Approach { target }));
    }
    if carrying {
        Ok(Some(InteractionStep::Wait))
    } else {
        Ok(Some(InteractionStep::Interact { target }))
    }
}

pub fn interact(world: &mut World, owner: Entity, target: Entity) -> Result<(), String> {
    let entry = world
        .get::<Npc>(target)
        .ok_or("interaction source lacks NPC")?
        .template_id;
    crate::npc_interaction::spellclick::cast_spellclick(world, owner, target, entry);
    // A channelled pickup reaches native cast success, and so the carry, at channel start.
    let pending = world
        .get::<crate::npc_interaction::nalorakk_offerings::PendingOffering>(owner)
        .is_some();
    let carrying = world
        .get::<crate::npc_interaction::nalorakk_offerings::CarryingOffering>(owner)
        .is_some();
    if !pending && !carrying {
        return Err(format!(
            "OBJECTIVE_INTERACTION_FAILED: npc{entry} did not start pickup"
        ));
    }
    Ok(())
}
