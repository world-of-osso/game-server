//! Holy257 specialization seam; native common triage until its policy lands.
use super::healer_triage::{SpecVerdict, TriageContext};

pub(super) fn decide(_ctx: &TriageContext<'_>) -> SpecVerdict {
    SpecVerdict::Defer { owned: &[] }
}
