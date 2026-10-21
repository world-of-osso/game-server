//! Restoration105 owns its healing rows; no return to the old HP ladder.
use shared::components::Health;

use super::healer_triage::{Decision, SpecVerdict, SpellSet, TriageContext};

const IRONBARK: u32 = 102342;
const SWIFTMEND: u32 = 18562;
const LIFEBLOOM: u32 = 33763;
const NATURES_SWIFTNESS: u32 = 132158;
const REGROWTH: u32 = 8936;
const NATURES_CURE: u32 = 88423;
const INNERVATE: u32 = 29166;
const TRANQUILITY: u32 = 740;
const REJUVENATION: u32 = 774;
const ABUNDANCE: u32 = 207383;
const ABUNDANCE_DISCOUNT: u32 = 207640;
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
    NATURES_SWIFTNESS,
    NATURES_CURE,
    INNERVATE,
];

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    let decision = choose_ironbark(ctx)
        .or_else(|| choose_swift_regrowth(ctx))
        .or_else(|| choose_urgent_swiftmend(ctx))
        .or_else(|| choose_tranquility(ctx))
        .or_else(|| choose_natures_swiftness(ctx))
        .or_else(|| choose_natures_cure(ctx))
        .or_else(|| choose_innervate(ctx))
        .or_else(|| choose_abundance_rejuvenation(ctx))
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
    if matches!(decision.spell_id, UTILITY_REBIRTH | TRANQUILITY) {
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

fn regrowth_target(ctx: &TriageContext<'_>) -> Option<Decision> {
    let entry = ctx
        .entries
        .iter()
        .find(|entry| entry.spell_id == REGROWTH)?;
    let patient = ctx.patients.iter().find(|patient| {
        patient.missing_health > 0.0
            && (patient.emergency || patient.score > 0.0)
            && super::kit_ai::patient_is_eligible(ctx.world, ctx.caster, patient.entity, entry)
    })?;
    Some(Decision {
        spell_id: REGROWTH,
        target: patient.entity,
        urgent: patient.emergency,
    })
}

fn choose_swift_regrowth(ctx: &TriageContext<'_>) -> Option<Decision> {
    ctx.own_aura_on(ctx.caster, NATURES_SWIFTNESS)?;
    if !ctx.ready(REGROWTH) {
        return None;
    }
    regrowth_target(ctx)
}

fn choose_natures_swiftness(ctx: &TriageContext<'_>) -> Option<Decision> {
    if !ctx.ready(NATURES_SWIFTNESS)
        || useful_friendly_cast_in_progress(ctx)
        || ctx.own_aura_on(ctx.caster, NATURES_SWIFTNESS).is_some()
    {
        return None;
    }
    let target = regrowth_target(ctx)?;
    if !target.urgent {
        return None;
    }
    let spell = ctx
        .world
        .resource::<crate::spell_info::SpellStore>()
        .get(REGROWTH)?;
    if crate::spell_cast::cast_time_ms(ctx.world, ctx.caster, spell) <= 0
        || crate::spell_cast::read_healer_recovery(ctx.world, ctx.caster, spell).is_err()
    {
        return None;
    }
    Some(Decision {
        spell_id: NATURES_SWIFTNESS,
        target: ctx.caster,
        urgent: true,
    })
}

fn useful_friendly_cast_in_progress(ctx: &TriageContext<'_>) -> bool {
    ctx.world
        .get::<shared::casting::CastState>(ctx.caster)
        .and_then(|cast| bevy::prelude::Entity::try_from_bits(cast.target))
        .is_some_and(|target| {
            !crate::spell_cast::targets::is_hostile(ctx.world, ctx.caster, target)
        })
}

fn choose_abundance_rejuvenation(ctx: &TriageContext<'_>) -> Option<Decision> {
    let threshold = ctx
        .own_aura_on(ctx.caster, ABUNDANCE)?
        .effects
        .iter()
        .find(|effect| effect.index == 0)?
        .amount;
    if !ctx.ready(REJUVENATION) || ctx.own_aura_on(ctx.caster, ABUNDANCE_DISCOUNT).is_some() {
        return None;
    }
    let active = ctx
        .patients
        .iter()
        .filter(|patient| ctx.own_aura_on(patient.entity, REJUVENATION).is_some())
        .count() as f32;
    if active >= threshold || active + 1.0 < threshold {
        return None;
    }
    ctx.candidates
        .iter()
        .find(|candidate| candidate.decision.spell_id == REJUVENATION)
        .map(|candidate| candidate.decision)
}

fn choose_tranquility(ctx: &TriageContext<'_>) -> Option<Decision> {
    let entry = ctx
        .entries
        .iter()
        .find(|entry| entry.spell_id == TRANQUILITY)?;
    if !ctx.ready(TRANQUILITY) || !channel_is_safe(ctx, TRANQUILITY) {
        return None;
    }
    let cluster = ctx.injury_cluster(TRANQUILITY, ctx.caster)?;
    let injured = cluster
        .members
        .iter()
        .filter_map(|unit| ctx.world.get::<Health>(*unit))
        .filter(|health| health.current < health.max * entry.conditions.health_below.unwrap_or(1.0))
        .count();
    if injured < entry.conditions.min_injured {
        return None;
    }
    Some(Decision {
        spell_id: TRANQUILITY,
        target: ctx.caster,
        urgent: false,
    })
}

fn channel_is_safe(ctx: &TriageContext<'_>, spell_id: u32) -> bool {
    let Some(spell) = ctx
        .world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
    else {
        return false;
    };
    let channel_seconds =
        crate::spell_cast::duration_ms(ctx.world, ctx.caster, spell) as f32 / 1000.0;
    ctx.patients
        .iter()
        .find(|patient| patient.entity == ctx.caster)
        .is_some_and(|patient| patient.projected_death_seconds > channel_seconds)
}

fn choose_innervate(ctx: &TriageContext<'_>) -> Option<Decision> {
    let low_mana = ctx
        .mana_fraction
        .is_some_and(|mana| mana <= super::healer_triage::LOW_MANA);
    let healing_needed = ctx
        .patients
        .iter()
        .any(|patient| patient.missing_health > 0.0);
    if !low_mana || !healing_needed || !ctx.ready(INNERVATE) {
        return None;
    }
    if ctx.own_aura_on(ctx.caster, INNERVATE).is_some() {
        return None;
    }
    Some(Decision {
        spell_id: INNERVATE,
        target: ctx.caster,
        urgent: false,
    })
}

fn choose_natures_cure(ctx: &TriageContext<'_>) -> Option<Decision> {
    let entry = ctx
        .entries
        .iter()
        .find(|entry| entry.spell_id == NATURES_CURE)?;
    if !ctx.ready(NATURES_CURE) {
        return None;
    }
    let patient = ctx.patients.iter().find(|patient| {
        super::kit_ai::patient_is_eligible(ctx.world, ctx.caster, patient.entity, entry)
    })?;
    Some(Decision {
        spell_id: NATURES_CURE,
        target: patient.entity,
        urgent: false,
    })
}
