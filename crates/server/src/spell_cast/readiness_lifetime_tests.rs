//! Reproduce native-equal-budget2's 400ms server frames with real elapsed time.
//! TC a352b1fa SpellHistory.cpp:834–855 uses GameTime for charge deadlines.
//! Keep Bevy's normal virtual-time cap: simulation catch-up is not this contract.
use std::time::{Duration, Instant};

use bevy::time::TimeUpdateStrategy;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

use super::test_harness::*;

struct ReadinessProbe {
    harness: Harness,
    spell: u32,
    recovery: Duration,
    requested_again: bool,
}

impl ReadinessProbe {
    fn start(spell: u32, class: u8, recovery_ms: u32) -> Self {
        let mut harness = harness(Vec::new(), &[spell], class).unwrap();
        harness.cast_and_deliver(spell, Some(harness.npc));
        assert!(
            harness.inbox.failed.is_empty(),
            "{:?}",
            harness.inbox.failed
        );
        let charge = harness.inbox.charges.last().expect("consumed owner charge");
        assert_eq!((charge.current, charge.max), (0, 1));
        assert_eq!(charge.recharge_ms, recovery_ms);
        harness.inbox.log.clear();
        harness.inbox.failed.clear();
        harness.app.insert_resource(TimeUpdateStrategy::Automatic);
        Self {
            harness,
            spell,
            recovery: Duration::from_millis(u64::from(recovery_ms)),
            requested_again: false,
        }
    }

    fn update(&mut self, elapsed: Duration) {
        self.harness.advance(1);
        // Let owner/intent delivery complete without relying on an exact frame rate.
        if !self.requested_again && elapsed >= self.recovery + Duration::from_secs(2) {
            self.harness.cast(self.spell, Some(self.harness.npc));
            self.requested_again = true;
        }
    }

    fn assert_recovered(&self) {
        assert!(self.requested_again);
        let successes: Vec<_> = self
            .harness
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(self.spell) && event.kind == CombatLogKind::CastSuccess
            })
            .collect();
        assert_eq!(
            successes.len(),
            1,
            "spell {} rejected after real recovery: {:?}",
            self.spell,
            self.harness.inbox.failed
        );
        assert!(
            self.harness
                .inbox
                .charges
                .iter()
                .any(|charge| charge.current == 1)
        );
    }
}

#[test]
fn native_readiness_real_sixty_second_owner_lifetime_survives_slow_frames() {
    // Active70391 SpellCategories1072/47749 → SpellCategory1680/1612.
    let mut overpower = ReadinessProbe::start(7384, 1, 12_000);
    let mut glaive = ReadinessProbe::start(185123, 12, 9_000);
    let started = Instant::now();
    let mut checked_gcd = false;
    while started.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(400));
        let elapsed = started.elapsed();
        overpower.update(elapsed);
        glaive.update(elapsed);
        if !checked_gcd && elapsed >= Duration::from_secs(2) {
            // The authored 1500ms GCD must have expired even though the charge has not.
            overpower.harness.cast(7384, Some(overpower.harness.npc));
            checked_gcd = true;
        }
    }
    assert!(checked_gcd);
    assert_eq!(
        overpower.harness.inbox.failed.len(),
        1,
        "only the intentional pre-recharge request may fail: {:?}",
        overpower.harness.inbox.failed
    );
    assert_eq!(
        overpower.harness.inbox.failed[0].reason,
        CastFailReason::NoChargesRemain
    );
    overpower.assert_recovered();
    glaive.assert_recovered();
}
