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
        65 => holy_paladin_rotation::action_spells(),
        66 => protection_paladin_rotation::action_spells(&profile(spec)?),
        70 => Ok(program_action_spells(
            &ret_rotation::read_program()?,
            &ret_rotation::bindings(),
        )),
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
        104 => Ok(program_action_spells(
            &guardian_tank_rotation::program()?,
            &guardian_tank_rotation::bindings(),
        )),
        105 => restoration_druid_rotation::action_spells(),
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
        256 => discipline_priest_rotation::action_spells(),
        257 => holy_priest_rotation::action_spells(),
        260 => Ok(program_action_spells(
            &outlaw_rotation::load_program(&profile(spec)?)?,
            &outlaw_rotation::bindings(),
        )),
        261 => Ok(program_action_spells(
            &subtlety_rotation::load_program(&profile(spec)?)?,
            &subtlety_rotation::bindings(),
        )),
        264 => restoration_shaman_rotation::action_spells(),
        269 => Ok(program_action_spells(
            &windwalker_rotation::source::load()?,
            &windwalker_rotation::bindings::build(),
        )),
        270 => mistweaver_rotation::action_spells(),
        577 => Ok(program_action_spells(
            &havoc_rotation::load_program()?,
            &havoc_rotation::bindings::build(),
        )),
        581 => Ok(program_action_spells(
            &vengeance_tank_rotation::program()?,
            &vengeance_tank_rotation::bindings(),
        )),
        1467 => Ok(program_action_spells(
            &devastation_rotation::source::load()?,
            &devastation_rotation::bindings::build(),
        )),
        1468 => preservation_evoker_rotation::action_spells(),
        1480 => Ok(program_action_spells(
            &devourer_rotation::load_program()?,
            &devourer_rotation::bindings::build(),
        )),
        _ => Err(format!("spec{spec} has no pure follower action metadata")),
    }
}
