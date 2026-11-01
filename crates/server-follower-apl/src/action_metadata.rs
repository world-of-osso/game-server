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
        64 => Ok(program_action_spells(
            &frost_mage_rotation::program(&profile(spec)?)?,
            &frost_mage_rotation::bindings::build(),
        )),
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
        251 => Ok(program_action_spells(
            &frost_rotation::read_program(&profile(spec)?)?,
            &frost_rotation::bindings::build(),
        )),
        252 => Ok(program_action_spells(
            &unholy_rotation::load_program(&profile(spec)?)?,
            &unholy_rotation::bindings(),
        )),
        253 => Ok(program_action_spells(
            &beast_mastery_rotation::program()?,
            &beast_mastery_rotation::bindings::build(),
        )),
        254 => Ok(program_action_spells(
            &marksmanship_rotation::program()?,
            &marksmanship_rotation::bindings::build(),
        )),
        255 => Ok(program_action_spells(
            &survival_rotation::program()?,
            &survival_rotation::bindings::build(),
        )),
        257 => holy_priest_rotation::action_spells(),
        260 => Ok(program_action_spells(
            &outlaw_rotation::load_program(&profile(spec)?)?,
            &outlaw_rotation::bindings(),
        )),
        261 => Ok(program_action_spells(
            &subtlety_rotation::load_program(&profile(spec)?)?,
            &subtlety_rotation::bindings(),
        )),
        577 => Ok(program_action_spells(
            &havoc_rotation::load_program()?,
            &havoc_rotation::bindings::build(),
        )),
        1480 => Ok(program_action_spells(
            &devourer_rotation::load_program()?,
            &devourer_rotation::bindings::build(),
        )),
        _ => Err(format!("spec{spec} has no pure follower action metadata")),
    }
}
