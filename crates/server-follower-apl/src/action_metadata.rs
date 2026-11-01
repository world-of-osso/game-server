//! Pure action metadata from the same programs and bindings used by the rotations.
use crate::class_progression::TraitLoadout;
use crate::literal_rotation::program_action_spells;
use crate::profiles::profile;
use bevy::prelude::{Entity, World};

/// Actor-specific inputs selected before pure program parsing.
pub struct ActionInputs<'a> {
    pub traits: &'a TraitLoadout,
    pub feral_berserk: Option<u32>,
    pub balance_alignment: u32,
    pub lunar_calling: bool,
    pub pet_summon: Option<u32>,
    pub coatings: Vec<u32>,
}

/// Read selected class actions without changing the actor or world.
pub fn read_inputs(world: &World, actor: Entity, spec: u32) -> Result<ActionInputs<'_>, String> {
    let traits = world
        .get::<TraitLoadout>(actor)
        .ok_or("follower traits unavailable")?;
    let feral_berserk = if spec == 103 {
        Some(
            crate::spell_cast::knowledge::resolve_action(world, actor, 106951)?
                .ok_or("native Feral reference lacks its selected Berserk/Incarnation action")?,
        )
    } else {
        None
    };
    let (balance_alignment, lunar_calling) = if spec == 102 {
        let alignment =
            crate::spell_cast::knowledge::resolve_action(world, actor, 194223)?.unwrap_or(194223);
        let lunar = world
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(429523));
        (alignment, lunar)
    } else {
        (194223, false)
    };
    let pet_summon = if matches!(spec, 265..=267) {
        crate::literal_rotation::default_pet::ConfiguredPet::read(world, actor, &profile(spec)?)?
            .map(|pet| pet.spell_id())
    } else {
        None
    };
    let coatings = if matches!(spec, 259 | 261) {
        crate::assassination_rotation::read_coatings(world, actor)
            .into_iter()
            .map(|action| action.spell)
            .collect()
    } else {
        Vec::new()
    };
    Ok(ActionInputs {
        traits,
        feral_berserk,
        balance_alignment,
        lunar_calling,
        pet_summon,
        coatings,
    })
}

/// Read a class's program-bound native action IDs without initializing its runtime.
pub fn action_spells(spec: u32, input: &ActionInputs<'_>) -> Result<Vec<u32>, String> {
    use crate::*;
    match spec {
        62 => Ok(program_action_spells(
            &arcane_rotation::program(&profile(spec)?)?,
            &arcane_rotation::bindings(),
        )),
        63 => fire_rotation::action_spells(&profile(spec)?, input.traits),
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
        102 => Ok(program_action_spells(
            &balance_rotation::read_program(&profile(spec)?)?,
            &balance_rotation::bindings::build_for_actions(
                input.balance_alignment,
                input.lunar_calling,
            ),
        )),
        103 => Ok(program_action_spells(
            &feral_rotation::source::load()?,
            &feral_rotation::bindings::build_for_berserk(
                input
                    .feral_berserk
                    .ok_or("Feral selected action unavailable")?,
            ),
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
        258 => Ok(program_action_spells(
            &shadow_rotation::program()?,
            &shadow_rotation::bindings::build(),
        )),
        259 => assassination_rotation::action_spells(
            &assassination_rotation::action_source(&profile(spec)?)?,
            &input.coatings,
        ),
        260 => Ok(program_action_spells(
            &outlaw_rotation::load_program(&profile(spec)?)?,
            &outlaw_rotation::bindings(),
        )),
        261 => Ok(
            crate::literal_rotation::program_action_spells_with_composite(
                &subtlety_rotation::load_program(&profile(spec)?)?,
                &subtlety_rotation::bindings(),
                "apply_poison",
                input.coatings.iter().copied(),
            ),
        ),
        262 => Ok(program_action_spells(
            &elemental_rotation::read_program(&profile(spec)?)?,
            &elemental_rotation::bindings::build(),
        )),
        263 => Ok(program_action_spells(
            &enhancement_rotation::read_program()?,
            &enhancement_rotation::bindings::build(),
        )),
        264 => restoration_shaman_rotation::action_spells(),
        265..=267 => {
            let mut bindings = match spec {
                265 => affliction_rotation::bindings::build(),
                266 => demonology_rotation::bindings::build(),
                267 => destruction_rotation::bindings::build(),
                _ => unreachable!(),
            };
            if let Some(spell) = input.pet_summon {
                if bindings
                    .actions
                    .get("summon_pet")
                    .is_some_and(|&existing| existing != spell)
                {
                    return Err(
                        "configured default pet conflicts with summon_pet native binding".into(),
                    );
                }
                bindings.actions.insert("summon_pet".into(), spell);
            }
            let profile = profile(spec)?;
            let program = match spec {
                265 => affliction_rotation::read_program(&profile)?,
                266 => demonology_rotation::read_program(&profile)?,
                267 => destruction_rotation::read_program(&profile)?,
                _ => unreachable!(),
            };
            Ok(program_action_spells(&program, &bindings))
        }
        268 => Ok(program_action_spells(
            &brewmaster_rotation::program(&profile(spec)?)?,
            &brewmaster_rotation::bindings(),
        )),
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
