//! Native70448 Fire Blast108853 Attr4/0x80; TC Spell.cpp530–531 ignores only
//! cast-in-progress, never the existing cast's completion, costs or cooldowns.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness};
use shared::protocol::CombatLogKind;

const FIREBALL: u32 = 133;
const FIRE_BLAST: u32 = 108853;
// Project spell queue window, spell_cast/mod.rs (user2026-10-15).
const PROJECT_QUEUE_WINDOW_SECS: f32 = 0.75;

fn assert_fire_blast_preserved_fireball(h: &mut Harness, caster: Entity, target: Entity) {
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.spell_id, FIREBALL);
    let duration = cast.duration;
    let elapsed = cast.elapsed;
    h.advance(1);
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.spell_id, FIREBALL);
    assert_eq!(cast.duration, duration);
    assert!(
        cast.elapsed > elapsed,
        "the original cast must keep progressing"
    );
    assert!(
        h.inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(caster.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(FIRE_BLAST)
        }),
        "Fire Blast must damage the target before Fireball completes"
    );
    h.advance(100);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIREBALL)
        }),
        "the original Fireball must still land"
    );
}

#[test]
fn dps_parity_cast_while_casting_native_creature_preserves_active_fireball() {
    let (mut h, caster) = crate::followers::kit_proc_fidelity_tests::prepare(209059);
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BLAST, Some(target), false)
        .expect("Fire Blast108853 must ignore only the cast-in-progress veto");
    assert_fire_blast_preserved_fireball(&mut h, caster, target);
}

#[test]
fn dps_parity_cast_while_casting_follower_kit_executes_without_self_interrupt() {
    let (mut h, caster) = crate::followers::kit_proc_fidelity_tests::prepare(209059);
    let target = h.npc;
    let follower = h.app.world().get::<Follower>(caster).unwrap().clone();
    let entry = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spec_id == 63 && entry.spell_id == FIRE_BLAST)
        .unwrap()
        .clone();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    assert!(
        attempt_cast(h.app.world_mut(), caster, target, &follower, &entry),
        "ordinary kit precheck must honor the same native cast attribute"
    );
    assert_fire_blast_preserved_fireball(&mut h, caster, target);
}

#[test]
fn dps_parity_cast_while_casting_player_requests_bypass_only_active_cast_delay() {
    for near_completion in [false, true] {
        let mut h = harness(Vec::new(), &[FIREBALL, FIRE_BLAST], 8).unwrap();
        let caster = h.caster;
        let target = h.npc;
        h.cast_and_deliver(FIREBALL, Some(target));
        if near_completion {
            // Enter the actual queue window from the original live cast clock;
            // do not fabricate a cast duration or remove the active CastState.
            for _ in 0..100 {
                let cast = h.app.world().get::<CastState>(caster).unwrap();
                if cast.duration - cast.elapsed < PROJECT_QUEUE_WINDOW_SECS / 2.0 {
                    break;
                }
                h.advance(1);
            }
        }
        h.cast_and_deliver(FIRE_BLAST, Some(target));
        assert!(
            h.inbox.failed.is_empty(),
            "native request: {:?}",
            h.inbox.failed
        );
        assert_fire_blast_preserved_fireball(&mut h, caster, target);
    }
}

#[test]
fn dps_parity_cast_while_casting_ordinary_second_cast_stays_blocked() {
    let (mut h, caster) = crate::followers::kit_proc_fidelity_tests::prepare(209059);
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    let failure =
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false);
    assert!(
        matches!(failure, Err(crate::spell_cast::CastFailure::InProgress)),
        "{failure:?}"
    );
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().spell_id,
        FIREBALL
    );
}
