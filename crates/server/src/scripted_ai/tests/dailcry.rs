//! LittleWigs2777a9d CaptainDailcry.lua; source207946, native70448.
//! Difficulty205 uses the existing approved Normal-population bridge.
use super::*;

const DAILCRY: u32 = 207946;
const MAP: u32 = 2649;
const BIT: u32 = 1;
const PIERCE: u32 = 424414;
const SPEAR: u32 = 1238780;
const BATTLE_CRY: u32 = 424419;
const MAULING: u32 = 447439;

fn dailcry_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(DAILCRY, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let mut near = *h.app.world().get::<Position>(boss).unwrap();
    near.x += 3.0;
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

fn assert_thirty_second_mauling_channel(h: &Harness, boss: Entity) {
    let cast = h
        .app
        .world()
        .get::<shared::casting::CastState>(boss)
        .expect("native Mauling channel remains active until its actual deadline");
    assert_eq!(cast.spell_id, MAULING);
    assert_eq!(cast.cast_type, shared::casting::CastType::Channel);
    assert_eq!(cast.duration, 30.0, "native70448 Attributes1 0x4/Duration9");
}

#[test]
fn encounter_coverage_dailcry_first_pierce_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = dailcry_copy(difficulty);
        h.engage(boss);
        h.advance(5.0);
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
            .expect("source Pierce Armor5.2s ordinary native cast");
        assert_eq!(cast.spell_id, PIERCE, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.5, "actual70448 native cast time");
    }
}

#[test]
fn encounter_coverage_dailcry_ongoing_native_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = dailcry_copy(difficulty);
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
        h.advance(14.9);
        assert_thirty_second_mauling_channel(&h, boss);
        h.advance(29.7);
        assert_thirty_second_mauling_channel(&h, boss);
        assert_eq!(
            h.app
                .world()
                .resource::<super::cast_admission::CastAdmissions>()
                .entries
                .len(),
            4,
            "native channel defers every ordinary outgoing lane"
        );
        h.advance(45.4);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let lanes = [
            (PIERCE, &[5.2, 44.8, 56.9, 69.0, 81.1][..]),
            (SPEAR, &[9.3, 47.3, 72.8][..]),
            (BATTLE_CRY, &[12.3, 50.3, 77.0][..]),
            (MAULING, &[14.8][..]),
        ];
        assert_eq!(
            attempts.len(),
            12,
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
        for spell in [PIERCE, BATTLE_CRY] {
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
                .any(|end| end.encounter_id == 2847 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
fn encounter_coverage_dailcry_evade_rearms_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = dailcry_copy(difficulty);
        h.engage(boss);
        h.advance(40.0);
        assert!(count_cast_receipts(&h, boss, PIERCE) > 0);
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
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        let before = count_cast_receipts(&h, boss, PIERCE);
        h.engage(boss);
        h.advance(5.0);
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
            PIERCE
        );
        h.advance(2.6);
        assert_eq!(count_cast_receipts(&h, boss, PIERCE), before + 1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
