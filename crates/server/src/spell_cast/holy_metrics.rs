//! Opt-in observations for the headless Holy comparison; no gameplay mutation.
use bevy::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub(crate) struct WordReduction {
    pub requests: u64,
    pub requested_seconds: f64,
    pub applied_seconds: f64,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct ActorMetrics {
    pub words: BTreeMap<u32, WordReduction>,
    pub surge_roll_successes: u64,
    pub surge_effective_gains: u64,
    pub surge_uses: u64,
}

#[derive(Resource, Debug, Default, Serialize)]
pub(crate) struct HolyMetrics {
    pub actors: BTreeMap<u64, ActorMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::test_harness::{Harness, harness};

    fn fixture() -> Harness {
        let mut h = harness(Vec::new(), &[2050, 2061], 5).unwrap();
        h.app.init_resource::<HolyMetrics>();
        h
    }

    fn observed(h: &Harness) -> Option<WordReduction> {
        h.app
            .world()
            .resource::<HolyMetrics>()
            .actors
            .get(&h.caster.to_bits())?
            .words
            .get(&2050)
            .copied()
    }

    #[test]
    fn holy_metrics_native_running_word_counts_requested_and_actual_seconds() {
        let mut h = fixture();
        h.cast_and_deliver(2050, Some(h.caster));
        h.advance(35);
        h.cast_and_deliver(2061, Some(h.caster));
        h.advance(45);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            observed(&h),
            Some(WordReduction {
                requests: 1,
                requested_seconds: 6.0,
                applied_seconds: 6.0,
            })
        );
    }

    #[test]
    fn holy_metrics_native_ready_word_records_zero_useful_reduction() {
        let mut h = fixture();
        h.cast_and_deliver(2061, Some(h.caster));
        h.advance(45);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            observed(&h),
            Some(WordReduction {
                requests: 1,
                requested_seconds: 6.0,
                applied_seconds: 0.0,
            })
        );
    }
}
