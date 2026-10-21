//! Restoration105 owns its healing rows; no return to the old HP ladder.
use shared::components::Health;

use super::healer_triage::{Decision, SpecVerdict, SpellSet, TriageContext};

const IRONBARK: u32 = 102342;
const SWIFTMEND: u32 = 18562;
const LIFEBLOOM: u32 = 33763;
// Matches the qualified native bloom-on-refresh window; durations remain native.
const LIFEBLOOM_REFRESH_FRACTION: f64 = 0.30;
// UTILITY_LAYER_REZ_PENDING: veto generic Rebirth; only the utility layer decides it.
const UTILITY_REBIRTH: u32 = 20484;
const OWNED: SpellSet = &[
    8936,
    18562,
    774,
    33763,
    48438,
    740,
    145205,
    IRONBARK,
    UTILITY_REBIRTH,
];

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    let decision = choose_ironbark(ctx)
        .or_else(|| choose_urgent_swiftmend(ctx))
        .or_else(|| choose_lifebloom(ctx))
        .or_else(|| ranked_maintenance(ctx));
    match decision {
        Some(decision) => SpecVerdict::Cast {
            decision,
            owned: OWNED,
        },
        None => SpecVerdict::Defer { owned: OWNED },
    }
}

fn choose_ironbark(ctx: &TriageContext<'_>) -> Option<Decision> {
    let entry = ctx
        .entries
        .iter()
        .find(|entry| entry.spell_id == IRONBARK)?;
    let threshold = entry.conditions.health_below?;
    if !ctx.ready(IRONBARK) {
        return None;
    }
    let target = ctx
        .patients
        .iter()
        .filter(|patient| {
            let below = ctx
                .world
                .get::<Health>(patient.entity)
                .is_some_and(|health| health.current < health.max * threshold);
            below
                && super::kit_ai::patient_is_eligible(ctx.world, ctx.caster, patient.entity, entry)
                && super::kit_ai::combat_allows(
                    ctx.world,
                    ctx.caster,
                    patient.entity,
                    &ctx.enemies,
                    entry,
                )
        })
        .min_by(|left, right| {
            left.projected_death_seconds
                .total_cmp(&right.projected_death_seconds)
        })?;
    Some(Decision {
        spell_id: IRONBARK,
        target: target.entity,
        urgent: true,
    })
}

fn choose_urgent_swiftmend(ctx: &TriageContext<'_>) -> Option<Decision> {
    let entry = ctx
        .entries
        .iter()
        .find(|entry| entry.spell_id == SWIFTMEND)?;
    let threshold = entry.conditions.health_below?;
    if !ctx.ready(SWIFTMEND) {
        return None;
    }
    let target = ctx.patients.iter().find(|patient| {
        ctx.world
            .get::<Health>(patient.entity)
            .is_some_and(|health| health.current < health.max * threshold)
            && super::kit_ai::patient_is_eligible(ctx.world, ctx.caster, patient.entity, entry)
    })?;
    Some(Decision {
        spell_id: SWIFTMEND,
        target: target.entity,
        urgent: true,
    })
}

fn ranked_maintenance(ctx: &TriageContext<'_>) -> Option<Decision> {
    let first = ctx.best_candidate(OWNED)?;
    if routine_heal_allowed(ctx, first) {
        return Some(first);
    }
    ctx.candidates
        .iter()
        .map(|candidate| candidate.decision)
        .find(|decision| OWNED.contains(&decision.spell_id) && routine_heal_allowed(ctx, *decision))
}

fn routine_heal_allowed(ctx: &TriageContext<'_>, decision: Decision) -> bool {
    if decision.spell_id == UTILITY_REBIRTH {
        return false;
    }
    if decision.spell_id != SWIFTMEND {
        return true;
    }
    // Do not spend the native owner-only prerequisite waiver for routine upkeep.
    // The urgent authored row above still uses the unmodified native target check.
    ctx.world
        .resource::<crate::spell_info::SpellStore>()
        .get(SWIFTMEND)
        .and_then(|spell| spell.aura_restrictions.as_ref())
        .is_some_and(|restriction| {
            crate::auras::aura_state::read_computed_state(
                ctx.world,
                decision.target,
                restriction.target_aura_state,
            ) == Some(true)
        })
}

fn choose_lifebloom(ctx: &TriageContext<'_>) -> Option<Decision> {
    if !ctx.ready(LIFEBLOOM) || ctx.patients.iter().any(|patient| patient.emergency) {
        return None;
    }
    let tank = ctx
        .patients
        .iter()
        .find(|patient| super::kit_ai::is_tank(ctx.world, patient.entity))?;
    let preparing = ctx.preparing(tank.entity);
    let threatened = tank.missing_health > 0.0 || tank.incoming_per_second > 0.0 || preparing;
    let aura = ctx.own_aura_on(tank.entity, LIFEBLOOM);
    if (!threatened && aura.is_none()) || (!threatened && ctx.enemies.is_empty()) {
        return None;
    }
    let due = aura.is_none_or(|aura| {
        aura.expires_at.is_some_and(|expiry| {
            expiry - crate::auras::aura_now(ctx.world)
                <= f64::from(aura.duration_ms) / 1000.0 * LIFEBLOOM_REFRESH_FRACTION
        })
    });
    due.then_some(Decision {
        spell_id: LIFEBLOOM,
        target: tank.entity,
        urgent: false,
    })
}
