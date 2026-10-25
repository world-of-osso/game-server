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

/// SimC76f8118 sc_mage.cpp6988–7010 sums caster-owned eligible travel events.
pub fn hot_streak_spells_in_flight(world: &World, caster: Entity) -> Result<f64, String> {
    let pending = world
        .get_resource::<missiles::PendingSpellHits>()
        .ok_or("native Hot Streak pending missile state unavailable")?;
    Ok(pending.count_matching(caster, scripts::triggers_hot_streak) as f64)
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
