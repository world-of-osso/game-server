//! Spec ID to production APL driver.
use crate::follower::SpecRotation;
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;

/// Specs whose class followers run their parity APL in production.
pub const SPECS: &[u32] = &[65, 70, 268];

/// Load the spec's driver for a pull starting now.
pub fn load(world: &mut World, actor: Entity, spec: u32) -> Result<Box<dyn SpecRotation>, String> {
    let encounter = Encounter::Dungeon {
        started_at: crate::spell_cast::now(world),
    };
    Ok(match spec {
        65 => Box::new(crate::holy_paladin_rotation::HolyPaladin::load(
            world, actor, encounter,
        )?),
        70 => Box::new(crate::ret_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        268 => Box::new(crate::brewmaster_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &crate::profiles::profile(268)?,
        )?),
        spec => return Err(format!("spec{spec} has no production APL driver")),
    })
}
