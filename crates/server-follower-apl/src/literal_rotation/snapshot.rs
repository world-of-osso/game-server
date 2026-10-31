//! Native stat sheet and power pools, read for SimC `snapshot_stats` and reports.
use bevy::prelude::*;
use serde_json::{Value, json};

pub fn stats(world: &World, actor: Entity) -> Result<Value, String> {
    let sheet = world
        .get::<crate::stats::CharacterStats>(actor)
        .ok_or("snapshot actor has no native stat sheet")?;
    let level = world
        .get::<shared::components::Level>(actor)
        .ok_or("snapshot actor has no level")?
        .0;
    let attack_time_mult = world
        .get::<crate::auras::AuraStatMods>(actor)
        .map_or(1.0, |mods| {
            mods.attack_cast_time_mult * mods.attack_time_mult * mods.melee_ranged_attack_time_mult
        });
    let auto_attack_speed = attack_time_mult / (1.0 + sheet.haste_pct / 100.0);
    Ok(
        json!({"attribute": {"strength": sheet.primary.strength, "agility": sheet.primary.agility,
        "stamina": sheet.primary.stamina, "intellect": sheet.primary.intellect},
        "native": {"crit_pct": sheet.crit_pct, "haste_pct": sheet.haste_pct,
        "mastery_pct": sheet.mastery_pct, "mastery_points": sheet.mastery_points,
        "versatility_pct": sheet.versatility_pct, "attack_power": sheet.attack_power,
        "ranged_attack_power": sheet.ranged_attack_power, "spell_power": sheet.spell_power,
        "armor": sheet.armor, "auto_attack_speed": auto_attack_speed}, "ratings": sheet.ratings,
        "main_hand": {"min": sheet.main_hand.min, "max": sheet.main_hand.max},
        "level": level, "powers": power_snapshot(world, actor)}),
    )
}

pub fn power_snapshot(world: &World, unit: Entity) -> Value {
    use crate::spell_cast::powers::{PowerPools, power_value};
    use shared::components::PowerType;
    let Some(pools) = world.get::<PowerPools>(unit) else {
        return Value::Null;
    };
    let values: Vec<_> = pools.order.iter().map(|&power| {
        let (current, max) = power_value(world, unit, power)
            .expect("native receipt pool order requires an authoritative value");
        let regen = if power == PowerType::Mana {
            pools.mana_regen_per_sec
        } else {
            pools.get(power).expect("native receipt non-mana pool exists").regen_per_sec
        };
        json!({"power_type": power as i32, "current": current, "max": max, "regen_per_sec": regen})
    }).collect();
    json!(values)
}
