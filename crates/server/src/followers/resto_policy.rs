//! Restoration105 owns its healing rows; no return to the old HP ladder.
use super::healer_triage::{SpecVerdict, SpellSet, TriageContext};

const OWNED: SpellSet = &[8936, 18562, 774, 33763, 48438, 740, 145205, 102342];

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    match ctx.best_candidate(OWNED) {
        Some(decision) => SpecVerdict::Cast {
            decision,
            owned: OWNED,
        },
        None => SpecVerdict::Defer { owned: OWNED },
    }
}
