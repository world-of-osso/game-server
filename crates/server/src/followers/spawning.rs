use bevy::prelude::*;
use shared::components::{Level, Mana, UnitFactionTemplate, UnitFlags, UnitLevel, UnitSummonedBy};

use super::{Follower, Profile};
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};

pub(super) fn spawn(world: &mut World, owner: Entity, profile: Profile) -> Result<Entity, String> {
    let position = *world
        .get::<shared::components::Position>(owner)
        .ok_or("follower owner has no position")?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let unit = crate::zones::summon_creature(world, owner, profile.entry, [x, y, z, 0.0])
        .ok_or_else(|| format!("authentic follower template {} unavailable", profile.entry))?;
    if let Err(error) = initialize(world, owner, unit, profile) {
        world.despawn(unit);
        return Err(error);
    }
    Ok(unit)
}

pub(super) fn initialize(
    world: &mut World,
    owner: Entity,
    unit: Entity,
    profile: Profile,
) -> Result<(), String> {
    let level = world
        .get::<Level>(owner)
        .ok_or("follower owner has no level")?
        .0;
    let faction = *world
        .get::<UnitFactionTemplate>(owner)
        .ok_or("follower owner has no faction")?;
    let flags = UnitFlags(
        world.get::<UnitFlags>(unit).map_or(0, |flags| flags.0)
            | crate::faction::UNIT_FLAG_PLAYER_CONTROLLED,
    );
    let (stats, pools, passives) = class_stats(world, &profile, level)?;
    let mana = Mana {
        current: stats.base_mana,
        max: stats.base_mana,
    };
    let health = shared::components::Health {
        current: stats.max_health,
        max: stats.max_health,
    };
    let form = profile.form_spell;
    world
        .entity_mut(unit)
        .remove::<crate::creature_aggro::AggroRange>()
        .insert((
            Follower { owner, profile },
            stats,
            pools,
            mana,
            health,
            UnitLevel(level),
            Level(level),
            faction,
            flags,
            UnitSummonedBy(owner.to_bits()),
            crate::creature_ai::FollowOwner::new(owner),
        ));
    crate::auras::sync_passives(world, unit, crate::auras::AuraSource::KnownSpell, &passives);
    if form != 0 {
        crate::spell_cast::creature_cast(world, unit, form, Some(unit), true)
            .map_err(|error| format!("follower preparation spell {form} failed: {error:?}"))?;
    }
    Ok(())
}

pub(super) fn class_stats(
    world: &World,
    profile: &Profile,
    level: u8,
) -> Result<(crate::stats::CharacterStats, PowerPools, Vec<u32>), String> {
    let classes = world.resource::<crate::spell_info::ClassSpells>();
    let class = classes
        .classes
        .iter()
        .find(|class| class.class_id == profile.class_id)
        .ok_or_else(|| format!("follower class {} absent", profile.class_id))?;
    let spec = class
        .specs
        .iter()
        .find(|spec| spec.spec_id == profile.spec_id)
        .ok_or_else(|| format!("follower specialization {} absent", profile.spec_id))?;
    let input = crate::stats::StatInputs {
        class: profile.class_id,
        race: 0,
        level,
        primary_stat: Some(crate::stats::PrimaryStat::from_priority(
            spec.primary_stat_priority,
        )),
        mastery_coefficient: 0.0,
        attack_power_per_strength: class.attack_power_per_strength,
        attack_power_per_agility: class.attack_power_per_agility,
        ranged_attack_power_per_agility: class.ranged_attack_power_per_agility,
        gear_primary: Default::default(),
        gear_ratings: Default::default(),
    };
    let stats = crate::stats::compute_character_stats(&input, None)
        .ok_or("unsupported follower class level")?;
    let table = world.resource::<PowerTypeTable>();
    let pools = PowerPools::from_table(
        table,
        table.class_powers(class.class_id, &class.power_types),
    );
    let passives = spec
        .spells
        .iter()
        .filter(|spell| spell.passive && spell.required_level <= u32::from(level))
        .map(|spell| spell.spell_id)
        .collect();
    Ok((stats, pools, passives))
}
