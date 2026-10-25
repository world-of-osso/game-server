//! Native cooldown and charge state reads, shared by casts' scripts and the APL; never mutate history.
use super::*;

struct Cooldown {
    remains: f64,
    duration: f64,
    charges: f64,
    max_charges: f64,
    fractional: f64,
    full_recharge: f64,
    started_at: Option<f64>,
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
    let source = if category_ready > spell_ready {
        history
            .and_then(|state| state.category_spell_ids.get(&category))
            .copied()
    } else {
        Some(spell.id)
    };
    let started_at = source.and_then(|source| {
        history
            .and_then(|state| state.cooldown_started_at.get(&source))
            .copied()
    });
    let remains = (spell_ready.max(category_ready) - now).max(0.0);
    let (spell_ms, category_ms) = base_cooldowns(spell);
    let duration_ms = cooldown_recovery_ms(world, caster, spell, spell_ms).max(if category != 0 {
        cooldown_recovery_ms(world, caster, spell, category_ms)
    } else {
        0
    });
    let charges = if remains == 0.0 { 1.0 } else { 0.0 };
    let duration = f64::from(duration_ms) / 1000.0;
    // SimC76f8118 cooldown.cpp660–680: ordinary cooldowns have one charge
    // whose fractional value advances between actual consumption and readiness.
    let fractional = if duration > 0.0 {
        (1.0 - remains / duration).clamp(0.0, 1.0)
    } else {
        charges
    };
    Cooldown {
        remains,
        duration,
        charges,
        max_charges: 1.0,
        fractional,
        full_recharge: remains,
        started_at,
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
    result.max_charges = f64::from(snapshot.max);
    result.duration = f64::from(snapshot.recharge_ms) / 1000.0;
    let next = (state.next_ready_at() - now).max(0.0);
    if snapshot.current == 0 {
        if next >= result.remains {
            result.started_at = Some(state.started_at);
        }
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

// SimC76f8118 cooldown.cpp603–615: estimate from the real start and the
// observed progress, including reductions; never replace it with raw remains.
fn expected_remains(state: &Cooldown, now: f64) -> Result<f64, String> {
    if state.remains == state.duration {
        return Ok(state.duration);
    }
    if state.remains == 0.0 && state.charges > 0.0 {
        return Ok(0.0);
    }
    let started_at = state
        .started_at
        .ok_or("native cooldown lacks its actual timer start")?;
    let reduction = (now - started_at) / (state.duration - state.remains);
    Ok(state.remains * reduction)
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
        "remains_expected" | "remains_guess" => expected_remains(&state, history_now(world)),
        "duration" => Ok(state.duration),
        "charges" => Ok(state.charges),
        // SimC76f8118 action.cpp3496–3500/cooldown.cpp: capacity, not remaining uses.
        "max_charges" => Ok(state.max_charges),
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
