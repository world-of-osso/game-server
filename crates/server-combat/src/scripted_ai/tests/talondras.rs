//! Current LittleWigs2777a9d SentinelTalondras.lua + native70448.
use super::*;

const TALONDRAS: u32 = 184124;
const MAP: u32 = 2451;
const BIT: u32 = 2;
const STOMP: u32 = 372701;
const SHARDS: u32 = 372718;

fn talondras_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(TALONDRAS, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|g| g.caster == boss.to_bits() && g.spell_id == spell)
        .count()
}

fn assert_native_admission(attempt: &super::cast_admission::Admission, expected_success: bool) {
    if expected_success {
        assert!(
            attempt.result.is_ok(),
            "ordinary native success {attempt:?}"
        );
    } else {
        assert!(
            matches!(
                &attempt.result,
                Err(crate::spell_cast::CastFailure::OnCooldown)
            ),
            "native1152 rejection {attempt:?}"
        );
    }
}

#[test]
fn encounter_coverage_talondras_native_stomp_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = talondras_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, STOMP, Some(boss), false)
            .expect("native372701");
        h.advance(2.5);
        assert_eq!(cast_count(&h, boss, STOMP), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(STOMP)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Stomp damage difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|e| e.source != Some(boss.to_bits()))
        );
    }
}

#[test]
fn encounter_coverage_talondras_native_shards_direct_and_periodic_qualify() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = talondras_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SHARDS, Some(player), false)
            .expect("native372718");
        h.advance(2.5);
        assert!(h.has_aura(player, SHARDS));
        for periodic in [false, true] {
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|e| e.kind == CombatLogKind::Damage
                        && e.amount > 0
                        && e.periodic == periodic
                        && e.spell_id == Some(SHARDS)
                        && e.source == Some(boss.to_bits())
                        && e.target == Some(player.to_bits())),
                "native direct/dot periodic={periodic} difficulty{difficulty}"
            );
        }
        assert!(!h.has_aura(h.players[1].0, SHARDS));
    }
}

#[test]
fn encounter_coverage_talondras_first_stomp_uses_current_lua_clock() {
    let (mut h, boss, _) = talondras_copy(205);
    let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .insert_resource(super::cast_admission::CastAdmissions {
            actor: boss,
            origin,
            entries: Vec::new(),
        });
    h.engage(boss);
    h.advance(8.0);
    let attempts = &h
        .app
        .world()
        .resource::<super::cast_admission::CastAdmissions>()
        .entries;
    let stomps: Vec<_> = attempts.iter().filter(|a| a.spell == STOMP).collect();
    assert_eq!(stomps.len(), 1, "current Lua5.1s request must exist");
    assert!((stomps[0].time - 5.1).abs() <= 0.1);
    assert!(
        matches!(
            &stomps[0].result,
            Err(crate::spell_cast::CastFailure::OnCooldown)
        ),
        "native shared1152 category is not bypassed"
    );
    assert_eq!(
        cast_count(&h, boss, STOMP),
        0,
        "TALONDRAS_LUA_DB2_CATEGORY_1152_CONFLICT"
    );
    h.advance(12.0);
    assert_eq!(
        cast_count(&h, boss, STOMP),
        1,
        "next sourced17.2s request is natively ready"
    );
}

#[test]
fn encounter_coverage_talondras_ongoing_native_lanes_damage_copy_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = talondras_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(105.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("TALONDRAS difficulty{difficulty} {attempts:?}");
        let lanes = [
            (
                SHARDS,
                &[
                    4.5, 14.2, 23.9, 33.6, 43.4, 53.1, 62.8, 72.5, 82.2, 91.9, 101.6,
                ][..],
                &[
                    true, true, false, false, false, true, true, true, false, false, true,
                ][..],
            ),
            (
                STOMP,
                &[5.1, 17.2, 29.3, 41.4, 53.5, 65.6, 77.7, 89.8, 101.9][..],
                &[false, true, true, true, false, false, true, true, false][..],
            ),
        ];
        assert_eq!(
            attempts.len(),
            20,
            "only current sourced Normal/Follower lanes"
        );
        // Parent-approved partial boundary: preserve native category1152 and
        // prove every acceptance/rejection, not20 fabricated successful casts.
        for (spell, times, accepted) in lanes {
            let actual: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(actual.len(), times.len(), "ongoing native lane{spell}");
            for ((attempt, time), accepted) in actual.iter().zip(times).zip(accepted) {
                assert_native_admission(attempt, *accepted);
                assert!(
                    (attempt.time - *time).abs() <= 0.15,
                    "native occupancy/source order {attempt:?} expected{time}"
                );
            }
            let successes = accepted.iter().filter(|&&value| value).count();
            assert_eq!(
                cast_count(&h, boss, spell),
                successes,
                "only accepted native requests emit SpellGo{spell}"
            );
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|e| e.kind == CombatLogKind::Damage
                        && e.amount > 0
                        && e.spell_id == Some(spell)
                        && e.source == Some(boss.to_bits())),
                "scheduled native damage{spell}"
            );
        }
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.periodic
                    && e.spell_id == Some(SHARDS)),
            "native scheduled periodic damage"
        );
        assert_eq!(
            cast_count(&h, boss, 372623),
            0,
            "Orb initial clock is not invented"
        );
        assert_eq!(
            cast_count(&h, boss, 372719),
            0,
            "BigWigs Normal includes205: no non-Normal Titanic initial"
        );
        assert!(
            h.has_aura(boss, 372600),
            "preserved actual Inexorable addon, not fabricated phase state"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|s| s.encounter_id == 2557 && s.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|g| g.caster != boss.to_bits())
        );
        h.hit_pct(boss, 200.0);
        h.advance(0.1);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(20.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "no new corpse casts");
    }
}

#[test]
fn encounter_coverage_talondras_evade_rearms_without_credit() {
    let (mut h, boss, map) = talondras_copy(205);
    h.engage(boss);
    h.advance(20.0);
    assert_eq!(cast_count(&h, boss, SHARDS), 2);
    assert_eq!(cast_count(&h, boss, STOMP), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    h.engage(boss);
    h.advance(4.2);
    assert_eq!(cast_count(&h, boss, SHARDS), 2, "no early reset Shards");
    h.advance(2.5);
    assert_eq!(
        cast_count(&h, boss, SHARDS),
        3,
        "native home/re-engage restores4.5s first Shards"
    );
}
