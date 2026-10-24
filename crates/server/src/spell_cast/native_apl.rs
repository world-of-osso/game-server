//! Read the same native cooldown, charge and GCD calculations used by casts.
use super::*;

/// Caster-owned actual travel; scripted Meteor contributes its live native continuation.
pub(crate) fn flight_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    property: &str,
) -> Result<f64, String> {
    let pending = world
        .get_resource::<missiles::PendingSpellHits>()
        .ok_or("native APL pending missile state unavailable")?;
    let direct = pending.earliest_due(caster, spell.id);
    let scripted = if spell.id == area_trigger::meteor::METEOR {
        area_trigger::meteor::pending_impact_due(world, caster)?
    } else {
        None
    };
    let due = direct.into_iter().chain(scripted).min_by(f64::total_cmp);
    match property {
        "in_flight" => Ok(if due.is_some() { 1.0 } else { 0.0 }),
        "in_flight_remains" => Ok(due.map_or(0.0, |at| (at - now(world)).max(0.0))),
        _ => Err(format!("unknown native APL flight property {property}")),
    }
}

/// Native target-only admission; no cooldown, power, knowledge or cast mutation.
pub(crate) fn target_eligible(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    candidate: Entity,
) -> Result<bool, String> {
    match validate_target(world, caster, spell, Some(candidate), Sight::Start) {
        Ok(target) => Ok(target == Some(candidate)),
        Err(CastFailure::Unsupported(reason)) => {
            Err(format!("native target eligibility unsupported: {reason}"))
        }
        Err(_) => Ok(false),
    }
}

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
        fractional,
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

/// SimC action.cpp3314–3333: channels occupy their duration plus cast time;
/// other actions occupy at least their native GCD.
pub(crate) fn execute_time(world: &World, caster: Entity, spell: &SpellInfo) -> f64 {
    let cast = f64::from(cast_time_ms(world, caster, spell).max(0)) / 1000.0;
    if is_channeled(spell) {
        cast + f64::from(duration_ms(world, caster, spell).max(0)) / 1000.0
    } else {
        cast.max(f64::from(gcd_duration_ms(world, caster, spell)) / 1000.0)
    }
}

/// Read-only action readiness, independent of the current GCD/executing action
/// (SimC action.cpp2640–2674). Actual dispatch still uses full native admission.
pub(crate) fn action_ready(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<f64, String> {
    match read_action_eligibility(world, caster, spell) {
        Ok(()) => Ok(1.0),
        Err(reason @ (CastFailure::UnknownSpell | CastFailure::Unsupported(_))) => Err(format!(
            "native APL action{} readiness unavailable: {reason:?}",
            spell.id
        )),
        Err(_) => Ok(0.0),
    }
}

fn read_action_eligibility(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    check_ordinary_caster_life(world, caster, spell)?;
    casting_restriction::check_cast(world, caster, spell)?;
    shapeshift::check_cast(world, caster, spell)?;
    aura_spell_restrictions::check_caster(world, caster, spell)?;
    if let Some(block) = crate::auras::cast_blocker(world, caster, spell) {
        return Err(CastFailure::CrowdControlled(block));
    }
    validate_caster_state(world, caster, spell)?;
    read_healer_recovery(world, caster, spell)?;
    let moving_blocks = (cast_time_ms(world, caster, spell) > 0
        && !cast_allows_moving(world, caster, spell))
        || (is_channeled(spell) && !channel_allows_moving(world, caster, spell));
    if moving_blocks && world.get::<Moving>(caster).is_some_and(|moving| moving.0) {
        return Err(CastFailure::Moving);
    }
    let explicit = world
        .get::<crate::networking::CombatTarget>(caster)
        .and_then(|target| target.0);
    let target = validate_target(world, caster, spell, explicit, Sight::Start)?;
    aura_spell_restrictions::check_target(world, caster, target, spell)?;
    spell_costs(world, caster, spell)?;
    Ok(())
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

pub fn gcd_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    property: &str,
) -> Result<f64, String> {
    match property {
        "gcd" | "gcd.max" => Ok(f64::from(gcd_duration_ms(world, caster, spell)) / 1000.0),
        "gcd.min" => Ok(if gcd_duration_ms(world, caster, spell) == 0 {
            0.0
        } else {
            f64::from(MIN_GCD_MS) / 1000.0
        }),
        "gcd.remains" => {
            let category = gcd_category(spell);
            let ready_at = world
                .get::<SpellHistory>(caster)
                .and_then(|history| history.gcd_ready_at.get(&category))
                .copied()
                .unwrap_or(0.0);
            Ok((ready_at - history_now(world)).max(0.0))
        }
        _ => Err(format!("unknown native APL GCD property {property}")),
    }
}
