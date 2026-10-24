//! Pinned BM dungeon APL active-enemy admission, not an area-radius threshold.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WILD_THRASH: u32 = 1264359;
const PET_THRASH: u32 = 1264355;
const COBRA_SHOT: u32 = 193455;

fn count_native_thrash(targets: usize) -> (usize, usize) {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        targets,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::followers::Follower>(run.actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(WILD_THRASH)
    );
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let owner_casts = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess
            })
            .count()
    };
    let pet_hits = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(PET_THRASH)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        owner_casts(COBRA_SHOT) > 0,
        "actual kit must fight, not pass by remaining idle"
    );
    eprintln!(
        "NATIVE_ACTIVE_ENEMIES targets={targets} wild_thrash={} pet_hits={pet_hits} cobra={}",
        owner_casts(WILD_THRASH),
        owner_casts(COBRA_SHOT)
    );
    (owner_casts(WILD_THRASH), pet_hits)
}

#[test]
#[ignore = "actual Austin kit with pinned253/current world; active-enemy ST RED"]
fn dps_parity_active_enemy_native_single_target_never_casts_wild_thrash() {
    let _seed = crate::random::seed_scope(70_448);
    let (casts, hits) = count_native_thrash(1);
    assert_eq!(casts, 0, "pinned st action requires active_enemies>1");
    assert_eq!(
        hits, 0,
        "do not hide a forbidden owner command behind a triggered child"
    );
}

#[test]
#[ignore = "actual Austin kit with two engaged targets; not a geometric radius gate"]
fn dps_parity_active_enemy_native_two_engaged_targets_keep_wild_thrash() {
    let _seed = crate::random::seed_scope(70_448);
    let (casts, hits) = count_native_thrash(2);
    assert!(
        casts > 0 && hits > 0,
        "ordinary native command and owned pet damage remain enabled"
    );
}

#[test]
#[ignore = "actual Austin kit with three engaged targets; retains AoE admission"]
fn dps_parity_active_enemy_native_three_engaged_targets_keep_wild_thrash() {
    let _seed = crate::random::seed_scope(70_448);
    let (casts, hits) = count_native_thrash(3);
    assert!(
        casts > 0 && hits > 0,
        "active-enemy admission must not reject radius-less command roots"
    );
}
