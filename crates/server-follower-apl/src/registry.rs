//! Spec ID to production APL driver.
use crate::follower::SpecRotation;
use crate::profiles::profile;
use crate::simc_apl_native::Encounter;
use bevy::prelude::*;

/// Specs whose class followers run their parity APL in production.
pub const SPECS: &[u32] = &[
    62, 63, 64, 65, 66, 70, 71, 72, 73, 102, 103, 104, 105, 250, 251, 252, 253, 254, 255, 256, 257,
    258, 259, 260, 261, 262, 263, 264, 265, 266, 267, 268, 269, 270, 577, 581, 1467, 1468, 1480,
];

/// Load the spec's driver for a pull starting now.
pub fn load(world: &mut World, actor: Entity, spec: u32) -> Result<Box<dyn SpecRotation>, String> {
    let encounter = Encounter::Dungeon {
        started_at: crate::spell_cast::now(world),
    };
    if matches!(spec, 253 | 255) {
        // SimC76f8118 sc_hunter.cpp1052: summon_pet defaults to the duck; Call Pet summons it.
        crate::hunter_pet::provision_follower_duck(world, actor)?;
    }
    use crate::*;
    Ok(match spec {
        62 => Box::new(arcane_rotation::Rotation::load(
            world,
            actor,
            &profile(62)?,
            encounter,
        )?),
        63 => Box::new(fire_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(63)?,
        )?),
        64 => Box::new(frost_mage_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(64)?,
        )?),
        65 => Box::new(holy_paladin_rotation::HolyPaladin::load(
            world, actor, encounter,
        )?),
        66 => Box::new(protection_paladin_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(66)?,
        )?),
        70 => Box::new(ret_rotation::Rotation::load(world, actor, encounter)?),
        71 => Box::new(arms_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(71)?,
        )?),
        72 => Box::new(fury_rotation::Rotation::load(world, actor, encounter)?),
        73 => Box::new(protection_warrior_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(73)?,
        )?),
        102 => Box::new(balance_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(102)?,
        )?),
        103 => Box::new(feral_rotation::Rotation::load(
            world,
            actor,
            &profile(103)?,
            encounter,
        )?),
        104 => Box::new(guardian_tank_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        105 => Box::new(restoration_druid_rotation::Restoration::load(
            world, actor, encounter,
        )?),
        250 => Box::new(blood_tank_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        251 => Box::new(frost_rotation::Rotation::load(
            world,
            actor,
            &profile(251)?,
            encounter,
        )?),
        252 => Box::new(unholy_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(252)?,
        )?),
        253 => Box::new(beast_mastery_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(253)?,
        )?),
        254 => Box::new(marksmanship_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(254)?,
        )?),
        255 => Box::new(survival_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(255)?,
        )?),
        256 => Box::new(discipline_priest_rotation::Discipline::load(
            world, actor, encounter,
        )?),
        257 => Box::new(holy_priest_rotation::Holy::load(world, actor, encounter)?),
        258 => Box::new(shadow_rotation::Rotation::load(world, actor, encounter)?),
        259 => Box::new(assassination_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &assassination_rotation::action_source(&profile(259)?)?,
        )?),
        260 => Box::new(outlaw_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(260)?,
        )?),
        261 => Box::new(subtlety_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(261)?,
        )?),
        262 => Box::new(elemental_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(262)?,
        )?),
        263 => Box::new(enhancement_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(263)?,
        )?),
        264 => Box::new(restoration_shaman_rotation::Restoration::load(
            world, actor, encounter,
        )?),
        265 => Box::new(affliction_rotation::Rotation::load(
            world,
            actor,
            &profile(265)?,
            encounter,
        )?),
        266 => Box::new(demonology_rotation::Rotation::load(
            world,
            actor,
            &profile(266)?,
            encounter,
        )?),
        267 => Box::new(destruction_rotation::Rotation::load(
            world,
            actor,
            &profile(267)?,
            encounter,
        )?),
        268 => Box::new(brewmaster_rotation::Rotation::load(
            world,
            actor,
            encounter,
            &profile(268)?,
        )?),
        269 => Box::new(windwalker_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        270 => Box::new(mistweaver_rotation::Mistweaver::load(
            world, actor, encounter,
        )?),
        577 => Box::new(havoc_rotation::Rotation::load(world, actor, encounter)?),
        581 => Box::new(vengeance_tank_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        1467 => Box::new(devastation_rotation::Rotation::load(
            world, actor, encounter,
        )?),
        1468 => Box::new(preservation_evoker_rotation::Preservation::load(
            world, actor, encounter,
        )?),
        1480 => Box::new(devourer_rotation::Rotation::load(world, actor, encounter)?),
        spec => return Err(format!("spec{spec} has no production APL driver")),
    })
}
