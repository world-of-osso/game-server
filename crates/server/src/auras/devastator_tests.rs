//! Native Devastator procs from real swings while Shield Slam's real cooldown runs.
use crate::spell_cast::{SpellHistory, history_now, test_harness::*};
use shared::protocol::CombatLogKind;

const DEVASTATOR: u32 = 236279;
const DAMAGE: u32 = 236282;
const SHIELD_SLAM: u32 = 23922;
const MARKER: u32 = 224324;

fn spawn_devastator() -> Harness {
    harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[DEVASTATOR, DAMAGE, SHIELD_SLAM, MARKER],
        1,
    )
    .unwrap()
}

fn child_hits(h: &Harness) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(DAMAGE)
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .count()
}

#[test]
fn devastator_reset_native_swings_reset_running_shield_slam_and_notify_owner() {
    let mut resets = 0;
    let mut hits = 0;
    // Sample the unmodified native 25% roll, not a fabricated chance endpoint.
    // 64 fresh actors have two unarmed swing opportunities each within 2.15s.
    for _ in 0..64 {
        let mut h = spawn_devastator();
        h.cast_and_deliver(SHIELD_SLAM, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(h.inbox.go.iter().any(|go| go.spell_id == SHIELD_SLAM));
        let ready_at = h
            .app
            .world()
            .get::<SpellHistory>(h.caster)
            .unwrap()
            .spell_ready_at[&SHIELD_SLAM];
        assert!(ready_at > history_now(h.app.world()));
        h.inbox.cooldowns.clear();
        h.attack(h.npc);
        h.advance(3 + 40);
        hits += child_hits(&h);
        let now = history_now(h.app.world());
        assert!(
            now < ready_at,
            "the original cooldown has not naturally expired"
        );
        let reset = h.inbox.cooldowns.iter().any(|update| {
            update.spell_id == SHIELD_SLAM && !update.is_gcd && update.remaining_ms == 0
        });
        let marked = h.inbox.log.iter().any(|event| {
            event.spell_id == Some(MARKER) && event.kind == CombatLogKind::AuraApplied
        });
        assert_eq!(
            marked, reset,
            "source marker accompanies the cooldown reset"
        );
        if reset {
            resets += 1;
            let ready = h
                .app
                .world()
                .get::<SpellHistory>(h.caster)
                .unwrap()
                .spell_ready_at
                .get(&SHIELD_SLAM)
                .is_none_or(|deadline| *deadline <= now);
            assert!(ready, "owner reset must reflect server readiness");
        }
    }
    assert!(
        hits >= 96,
        "native landed Devastator procs observed: {hits}"
    );
    assert!(
        resets > 0 && resets < 64,
        "native25% proc should produce both reset and non-reset trials: {resets}/64 from{hits}hits"
    );
}

#[test]
fn devastator_reset_ready_shield_slam_keeps_default_damage_without_marker() {
    let mut h = spawn_devastator();
    h.attack(h.npc);
    h.advance(3 + 9 * 40);
    assert!(
        child_hits(&h) > 0,
        "the existing native damage proc remains active"
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|update| update.spell_id == SHIELD_SLAM)
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(MARKER))
    );
}
