use super::Catalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub health: i32,
    pub power: i32,
    pub speed: i32,
}

/// Raw state scale and offsets: TrinityCore a352b1fa BattlePet::CalculateStats.
/// Multipliers are supplied by the local BattlePetBreedQuality table, not constants.
pub fn calculate_stats(
    catalog: &Catalog,
    species: u32,
    breed: u32,
    quality: u32,
    level: u8,
) -> Result<Stats, String> {
    if !(1..=25).contains(&level) {
        return Err(format!("invalid pet level {level}"));
    }
    let species = catalog
        .species_states
        .get(&species)
        .ok_or_else(|| format!("missing species states {species}"))?;
    let breed = catalog
        .breed_states
        .get(&breed)
        .ok_or_else(|| format!("missing breed states {breed}"))?;
    let multiplier = catalog
        .qualities
        .get(&quality)
        .ok_or_else(|| format!("missing quality {quality}"))?;
    let scale = |name: &str, divisor: f32| -> Result<i32, String> {
        let state = catalog.state_id(name)?;
        let base = breed
            .get(&state)
            .ok_or_else(|| format!("breed missing {name}"))?;
        let raw = base + species.get(&state).copied().unwrap_or(0);
        if raw <= 0 {
            return Err(format!("nonpositive base stat {name}: {raw}"));
        }
        // DB2 multiplier is float32; preserve client/runtime precision at half boundaries.
        Ok((raw as f32 * *multiplier as f32 * f32::from(level) / divisor).round() as i32)
    };
    Ok(Stats {
        health: scale("Stat_Stamina", 20.0)? + 100,
        power: scale("Stat_Power", 100.0)?,
        speed: scale("Stat_Speed", 100.0)?,
    })
}
