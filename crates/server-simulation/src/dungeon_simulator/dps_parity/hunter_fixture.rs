//! Native default-pet record for explicit Hunter actors.
use bevy::prelude::*;

pub(super) fn require_profile_pet(world: &World, actor: Entity) -> Result<(), String> {
    let entry = world
        .get::<crate::hunter_pet::PetStable>(actor)
        .and_then(|stable| stable.stored.active[0].as_ref())
        .map(|pet| pet.creature_entry)
        .ok_or("hunter loadout has no active pet")?;
    if entry != 174884 {
        return Err(format!(
            "hunter loadout pet {entry} is not the SimC default duck"
        ));
    }
    Ok(())
}
