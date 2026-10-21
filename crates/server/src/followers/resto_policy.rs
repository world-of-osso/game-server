//! Restoration105 owns its healing rows; no return to the old HP ladder.
use shared::components::Health;

use super::healer_triage::{Decision, SpecVerdict, SpellSet, TriageContext};

const IRONBARK: u32 = 102342;
const OWNED: SpellSet = &[8936, 18562, 774, 33763, 48438, 740, 145205, IRONBARK];

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    let decision = choose_ironbark(ctx).or_else(|| ctx.best_candidate(OWNED));
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
