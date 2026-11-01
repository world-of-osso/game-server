//! Pure action metadata from the same programs and bindings used by the rotations.
use crate::class_progression::TraitLoadout;
use crate::literal_rotation::program_action_spells;
use crate::profiles::profile;

/// Read a class's program-bound native action IDs without initializing its runtime.
pub fn action_spells(spec: u32, traits: &TraitLoadout) -> Result<Vec<u32>, String> {
    use crate::*;
    match spec {
        62 => Ok(program_action_spells(
            &arcane_rotation::program(&profile(spec)?)?,
            &arcane_rotation::bindings(),
        )),
        63 => fire_rotation::action_spells(&profile(spec)?, traits),
        66 => protection_paladin_rotation::action_spells(&profile(spec)?),
        71 => Ok(program_action_spells(
            &arms_rotation::program(&profile(spec)?)?,
            &arms_rotation::bindings(),
        )),
        72 => Ok(program_action_spells(
            &fury_rotation::program()?,
            &fury_rotation::bindings(),
        )),
        73 => Ok(program_action_spells(
            &protection_warrior_rotation::program(&profile(spec)?)?,
            &protection_warrior_rotation::bindings(),
        )),
        250 => Ok(program_action_spells(
            &blood_tank_rotation::program()?,
            &blood_tank_rotation::bindings(),
        )),
        257 => holy_priest_rotation::action_spells(),
        _ => Err(format!("spec{spec} has no pure follower action metadata")),
    }
}
