//! Native cooldown and charge state reads, shared by casts' scripts and the APL; never mutate history.
use super::*;

struct Cooldown {
    remains: f64,
    duration: f64,
    charges: f64,
    fractional: f64,
    full_recharge: f64,
}

fn ordinary_cooldown(world: &World, caster: Entity, spell: &SpellInfo) -> Cooldown {
    let now = history_now(world);
    let category = spell.categories.as_ref().map_or(0, |row| row.category);
    let history = world.get::<SpellHistory>(caster);
    let spell_ready = history
        .and_then(|state| state.spell_ready_at.get(&spell.id))
        .copied()
        .unwrap_or(0.0);
    let category_ready = history
        .and_then(|state| state.category_ready_at.get(&category))
        .copied()
        .unwrap_or(0.0);
    let remains = (spell_ready.max(category_ready) - now).max(0.0);
    let (spell_ms, category_ms) = base_cooldowns(spell);
    let duration_ms = cooldown_recovery_ms(world, caster, spell, spell_ms).max(if category != 0 {
        cooldown_recovery_ms(world, caster, spell, category_ms)
    } else {
        0
    });
    let charges = if remains == 0.0 { 1.0 } else { 0.0 };
    Cooldown {
        remains,
        duration: f64::from(duration_ms) / 1000.0,
        charges,
        fractional: charges,
        full_recharge: remains,
    }
}

fn read_cooldown(world: &World, caster: Entity, spell: &SpellInfo) -> Cooldown {
    let mut result = ordinary_cooldown(world, caster, spell);
    let now = history_now(world);
    // A first-use category starts fully charged in the native admission path too.
    // Refresh only this read clone; querying facts cannot consume or retime casts.
    let mut history = world
        .get::<SpellHistory>(caster)
        .cloned()
        .unwrap_or_default();
    let Some(state) = history.charges_mut(spell, now, charge_mods(world, caster, spell)) else {
        return result;
    };
    let snapshot = state.snapshot(now);
    result.charges = f64::from(snapshot.current);
    result.duration = f64::from(snapshot.recharge_ms) / 1000.0;
    let next = (state.next_ready_at() - now).max(0.0);
    if snapshot.current == 0 {
        result.remains = result.remains.max(next);
    }
    let full = state
        .pending
        .back()
        .map_or(0.0, |charge| (charge.ready_at - now).max(0.0));
    result.full_recharge = result.remains.max(full);
    let partial = if state.pending.is_empty() || result.duration <= 0.0 {
        0.0
    } else {
        1.0 - next / result.duration
    };
    result.fractional = (result.charges + partial).min(f64::from(snapshot.max));
    result
}

pub(crate) fn cooldown_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    property: &str,
) -> Result<f64, String> {
    let state = read_cooldown(world, caster, spell);
    match property {
        "remains" => Ok(state.remains),
        "duration" => Ok(state.duration),
        "charges" => Ok(state.charges),
        "charges_fractional" => Ok(state.fractional),
        "full_recharge_time" => Ok(state.full_recharge),
        "up" | "ready" => Ok(if state.remains == 0.0 && state.charges > 0.0 {
            1.0
        } else {
            0.0
        }),
        _ => Err(format!("unknown native APL cooldown property {property}")),
    }
}
