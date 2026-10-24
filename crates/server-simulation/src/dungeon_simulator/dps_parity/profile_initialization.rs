//! Declared profile starting values are configuration, never post-cast energizes.
use super::inputs::Profile;
use bevy::prelude::*;
use rusqlite::OptionalExtension;
use shared::components::{Mana, PowerType};

pub(super) fn initialize_declared_powers(
    world: &mut World,
    actor: Entity,
    profile: &Profile,
) -> Result<(), String> {
    let values = read_declared_powers(profile)?;
    let values = values
        .into_iter()
        .map(|(power, displayed)| validate_starting_power(world, actor, power, displayed))
        .collect::<Result<Vec<_>, _>>()?;
    for (power, raw) in values {
        if power == PowerType::Mana {
            world
                .get_mut::<Mana>(actor)
                .ok_or("reference actor lacks native Mana")?
                .current = raw;
        } else {
            let mut pools = world
                .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
                .ok_or("reference actor lacks native PowerPools")?;
            let pool = pools
                .get_mut(power)
                .ok_or_else(|| format!("reference actor lacks native {power:?}"))?;
            pool.current = raw;
        }
    }
    Ok(())
}

fn read_declared_powers(profile: &Profile) -> Result<Vec<(PowerType, f32)>, String> {
    let class = profile
        .options
        .get("class")
        .and_then(serde_json::Value::as_str)
        .ok_or("reference profile lacks its class namespace")?;
    let prefix = format!("{class}.");
    let options: Vec<_> = profile
        .options
        .iter()
        .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|name| (name, value)))
        .filter(|(name, _)| *name != "default_pet")
        .collect();
    if options.is_empty() {
        return Ok(Vec::new());
    }
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("DPS_PARITY_WORLD_DB absent")?;
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("read native starting-power bindings: {error}"))?;
    options
        .into_iter()
        .map(|(name, value)| read_starting_power(&db, name, value))
        .collect()
}

fn read_starting_power(
    db: &rusqlite::Connection,
    name: &str,
    value: &serde_json::Value,
) -> Result<(PowerType, f32), String> {
    let id: Option<i32> = db
        .query_row(
            "SELECT PowerTypeEnum FROM power_type WHERE NameGlobalStringTag=upper(?1)",
            [name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("read native power {name}: {error}"))?;
    let power = id
        .and_then(PowerType::from_db)
        .ok_or_else(|| format!("unsupported reference class option {name}"))?;
    let displayed = value
        .as_str()
        .ok_or_else(|| format!("reference {name} must be a numeric profile string"))?
        .parse::<f32>()
        .map_err(|error| format!("reference initial {name}: {error}"))?;
    Ok((power, displayed))
}

fn validate_starting_power(
    world: &World,
    actor: Entity,
    power: PowerType,
    displayed: f32,
) -> Result<(PowerType, f32), String> {
    let info = world
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(power)
        .ok_or_else(|| format!("native starting-power metadata absent: {power:?}"))?;
    let (_, maximum) = crate::spell_cast::powers::power_value(world, actor, power)
        .ok_or_else(|| format!("reference actor lacks declared native power {power:?}"))?;
    let raw = displayed * info.display_modifier as f32;
    if info.display_modifier <= 0
        || !raw.is_finite()
        || raw < info.min_power as f32
        || raw > maximum
    {
        return Err(format!(
            "reference initial {power:?}={displayed} is outside native raw bounds {}..{maximum} with display divisor {}",
            info.min_power, info.display_modifier
        ));
    }
    Ok((power, raw))
}
