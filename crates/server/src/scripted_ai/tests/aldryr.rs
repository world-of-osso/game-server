//! LittleWigs2777a9d BrewMasterAldryr.lua; source210271, native70448.
//! Difficulty205 uses the existing approved Normal-population bridge.
use super::*;

const ALDRYR: u32 = 210271;
const MAP: u32 = 2661;
const BIT: u32 = 0;
const KEG_SMASH: u32 = 432229;
const THROW: u32 = 432179;
const BELCH: u32 = 432198;

fn aldryr_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ALDRYR, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    // Place test players in the actual source actor's forward cone, never rotate the actor.
    let yaw = h.app.world().get::<Rotation>(boss).unwrap().y;
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let near = Position {
        x: origin.x + 3.0 * yaw.sin(),
        z: origin.z + 3.0 * yaw.cos(),
        ..origin
    };
    let player = h.player();
    let peer = h.players[1].0;
    h.app.world_mut().entity_mut(player).insert(near);
    h.app.world_mut().entity_mut(peer).insert(near);
    (h, boss, map)
}

fn count_cast_receipts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
#[ignore = "native Aldryr source proof pending"]
fn encounter_coverage_aldryr_first_keg_smash_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = aldryr_copy(difficulty);
        h.engage(boss);
        h.advance(4.9);
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(boss)
            .expect("source Keg Smash5.1s ordinary native cast");
        assert_eq!(cast.spell_id, KEG_SMASH, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.5, "actual70448 native cast time");
    }
}

#[test]
#[ignore = "native Aldryr ongoing proof pending"]
fn encounter_coverage_aldryr_ongoing_native_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = aldryr_copy(difficulty);
        let maps = h.app.world().resource::<MapStore>().clone();
        let peer_map = h
            .app
            .world_mut()
            .resource_mut::<crate::maps::InstanceManager>()
            .create_map(
                &maps,
                MAP,
                crate::maps::instances::InstanceEntrant {
                    character_id: 2002,
                    leader: 2002,
                    difficulty_id: difficulty,
                    raid_difficulty: crate::maps::RaidDifficulty {
                        primary: difficulty,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                crate::maps::instances::Now {
                    elapsed: 0.0,
                    unix: 0,
                },
            )
            .unwrap()
            .map;
        let peer = h.players[1].0;
        h.app
            .world_mut()
            .entity_mut(peer)
            .insert((peer_map, crate::persistence::ClientId(2002)));
        assert_ne!(peer_map, map);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(90.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let lanes = [
            (KEG_SMASH, &[5.1, 19.6, 34.1, 48.6, 63.4, 77.9][..]),
            (THROW, &[10.0, 28.2, 46.4, 64.6, 82.8][..]),
            (BELCH, &[14.4, 37.4, 60.4, 84.3][..]),
        ];
        assert_eq!(
            attempts.len(),
            15,
            "all source/native occupancy requests: {attempts:?}"
        );
        for (spell, times) in lanes {
            let lane: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(lane.len(), times.len(), "source lane{spell}: {lane:?}");
            for (attempt, expected) in lane.iter().zip(times) {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "source/native occupancy {attempt:?}, expected{expected}"
                );
                assert!(attempt.result.is_ok(), "ordinary admission {attempt:?}");
            }
            assert_eq!(count_cast_receipts(&h, boss, spell), times.len());
        }
        for spell in [KEG_SMASH, BELCH] {
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|event| event.kind == CombatLogKind::Damage
                        && event.amount > 0
                        && event.spell_id == Some(spell)
                        && event.source == Some(boss.to_bits())
                        && event.target == Some(h.player().to_bits())),
                "native damage{spell}, difficulty{difficulty}"
            );
        }
        assert!(h.inbox(1).spell_gos.is_empty());
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        let player = h.player();
        crate::combat::kill_unit(h.app.world_mut(), player, boss);
        h.advance(0.1);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(peer_map),
            0
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == 2900 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
#[ignore = "native Aldryr evade/rearm proof pending"]
fn encounter_coverage_aldryr_evade_rearms_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = aldryr_copy(difficulty);
        h.engage(boss);
        h.advance(40.0);
        assert!(count_cast_receipts(&h, boss, KEG_SMASH) > 0);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(5.0);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        let casts = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "evade retires old lanes");
        let yaw = h.app.world().get::<Rotation>(boss).unwrap().y;
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + 3.0 * yaw.sin(),
            z: origin.z + 3.0 * yaw.cos(),
            ..origin
        });
        let before = count_cast_receipts(&h, boss, KEG_SMASH);
        h.engage(boss);
        h.advance(4.9);
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .unwrap()
                .spell_id,
            KEG_SMASH
        );
        h.advance(1.6);
        assert_eq!(count_cast_receipts(&h, boss, KEG_SMASH), before + 1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
