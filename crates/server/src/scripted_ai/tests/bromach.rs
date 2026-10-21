//! Current LittleWigs2777a9d Bromach.lua, native70448; no legacy alternate.
use super::*;

const BROMACH: u32 = 184018;
const MAP: u32 = 2451;
const BIT: u32 = 1;
const CALL: u32 = 369605;
const SLAM: u32 = 369703;

fn bromach_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(BROMACH, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let wdt = h.app.world().resource::<MapStore>().maps[&MAP].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(MAP, wdt)]),
    ));
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|g| g.caster == boss.to_bits() && g.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_bromach_native_slam_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = bromach_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SLAM, Some(boss), false)
            .expect("native369703 admission");
        h.advance(4.0);
        assert_eq!(cast_count(&h, boss, SLAM), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(SLAM)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Slam area damage difficulty{difficulty}"
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
#[ignore = "RED: current Bromach Lua-clock port absent"]
fn encounter_coverage_bromach_first_call_of_the_deep_uses_current_lua_clock() {
    let (mut h, boss, _) = bromach_copy(205);
    h.engage(boss);
    h.advance(6.0);
    assert_eq!(
        cast_count(&h, boss, CALL),
        1,
        "current Lua5.1s request, not melee-only"
    );
}

#[test]
fn encounter_coverage_bromach_ongoing_source_lanes_native_totems_damage_and_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = bromach_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(140.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("BROMACH difficulty{difficulty} {attempts:?}");
        let lanes = [
            (CALL, &[5.1, 36.4, 74.2, 103.3][..]),
            (SLAM, &[12.1, 32.9, 64.4, 82.6, 114.1, 133.6][..]),
            (382303, &[20.4, 50.7, 90.8, 121.1][..]),
            (369754, &[27.9, 58.2, 98.3, 128.6][..]),
        ];
        assert_eq!(attempts.len(), 18, "only current Lua/native requests");
        for (spell, times) in lanes {
            let actual: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(actual.len(), times.len(), "ongoing native lane{spell}");
            for (attempt, time) in actual.iter().zip(times) {
                assert!(attempt.result.is_ok(), "ordinary admission {attempt:?}");
                assert!(
                    (attempt.time - *time).abs() <= 0.15,
                    "source-order/native cast occupancy {attempt:?} expected{time}"
                );
            }
            assert_eq!(
                cast_count(&h, boss, spell),
                times.len(),
                "native SpellGo{spell}"
            );
        }
        let totems = h.creatures_of(186696);
        assert_eq!(totems.len(), 4, "actual native382303→382302→186696 chain");
        for totem in totems {
            assert_eq!(*h.app.world().get::<InMap>(totem).unwrap(), map);
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::zones::TempSummon>(totem)
                    .unwrap()
                    .summoner,
                boss
            );
            assert!(h.has_aura(totem, 388314), "actual authored template addon");
            assert!(h.health(totem).max > 0.0);
        }
        assert!(h.has_aura(boss, 369754), "native scheduled Bloodlust aura");
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(SLAM)
                    && e.source == Some(boss.to_bits())),
            "scheduled native Slam damage"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|s| s.encounter_id == 2556 && s.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
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
        assert!(
            h.creatures_of(186696).is_empty(),
            "native summon list cleans up on death"
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(45.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
fn encounter_coverage_bromach_cancelled_slam_has_no_success_repeat() {
    let (mut h, boss, _) = bromach_copy(205);
    h.engage(boss);
    h.advance(12.3);
    assert_eq!(h.app.world().get::<CastState>(boss).unwrap().spell_id, SLAM);
    crate::spell_cast::cancel_cast(h.app.world_mut(), boss);
    h.advance(80.0);
    assert_eq!(
        cast_count(&h, boss, SLAM),
        0,
        "Lua success-only repeat is never armed after interruption"
    );
    assert!(
        cast_count(&h, boss, CALL) >= 2,
        "other authored lanes continue"
    );
}

#[test]
fn encounter_coverage_bromach_evade_cleans_totems_and_rearms_first_call() {
    let (mut h, boss, map) = bromach_copy(205);
    h.engage(boss);
    h.advance(25.0);
    assert_eq!(h.creatures_of(186696).len(), 1);
    assert_eq!(cast_count(&h, boss, CALL), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert!(h.creatures_of(186696).is_empty());
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
    h.advance(4.8);
    assert_eq!(cast_count(&h, boss, CALL), 1, "no early reset Call");
    h.advance(1.0);
    assert_eq!(cast_count(&h, boss, CALL), 2);
    h.advance(17.0);
    assert_eq!(
        h.creatures_of(186696).len(),
        1,
        "reset resumes source totem/native summon chain"
    );
}
