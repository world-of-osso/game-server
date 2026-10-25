//! Pure specialization hook. Ownership survives a refused native cast.
use super::healer_triage::{SpecVerdict, TriageContext};

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    match ctx.follower.profile.spec_id {
        105 => super::resto_policy::decide(ctx),
        257 => super::holy_policy::decide(ctx),
        _ => SpecVerdict::Defer { owned: &[] },
    }
}
