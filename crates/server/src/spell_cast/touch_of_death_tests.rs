//! Source70391 primary-target admission, before costs and cooldowns.
use super::test_harness::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TOUCH_OF_DEATH: u32 = 322109;
const IMPROVED_TOUCH_OF_DEATH: u32 = 322113;

fn cast_at_health(current: f32, maximum: f32, improved: bool) -> Harness {
    let ids = if improved {
        vec![TOUCH_OF_DEATH, IMPROVED_TOUCH_OF_DEATH]
    } else {
        vec![TOUCH_OF_DEATH]
    };
    let mut h = harness(Vec::new(), &ids, 10).unwrap();
    h.advance(3);
    assert_eq!(
        crate::auras::has_aura(h.app.world(), h.caster, IMPROVED_TOUCH_OF_DEATH),
        improved
    );
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current,
        max: maximum,
    });
    h.cast_and_deliver(TOUCH_OF_DEATH, Some(h.npc));
    h
}

#[test]
fn touch_of_death_native_ineligible_target_rejects_without_success_or_cooldown() {
    for improved in [false, true] {
        let h = cast_at_health(5000.0, 5000.0, improved);
        assert!(
            h.inbox.failed.iter().any(|failure| {
                failure.spell_id == TOUCH_OF_DEATH
                    && failure.reason == CastFailReason::InvalidTarget
            }),
            "healthy stronger target must fail: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.log.iter().all(|event| {
            event.spell_id != Some(TOUCH_OF_DEATH) || event.kind != CombatLogKind::CastSuccess
        }));
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .all(|event| event.spell_id != TOUCH_OF_DEATH)
        );
        assert_eq!(h.health(h.npc), 5000.0);
    }
}

#[test]
fn touch_of_death_native_stronger_creature_needs_improved_rank_and_below_threshold() {
    for (current, improved, allowed) in [
        (1400.0, false, false),
        (1400.0, true, true),
        (1500.0, true, false),
        (1600.0, true, false),
        (999.0, false, true),
    ] {
        let h = cast_at_health(current, 10000.0, improved);
        let succeeded = h.inbox.log.iter().any(|event| {
            event.spell_id == Some(TOUCH_OF_DEATH) && event.kind == CombatLogKind::CastSuccess
        });
        assert_eq!(
            succeeded, allowed,
            "health={current}, improved={improved}, failures={:?}",
            h.inbox.failed
        );
    }
}
