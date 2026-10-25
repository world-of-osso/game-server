//! TC a352b1fa boss_the_candle_king.cpp; native70448 source actor208745.
//! Follower205 actor is explicitly projected from the authored Normal placement;
//! no205 spawn, stats or difficulty rows are fabricated or published.
use super::*;

const CANDLE_KING: u32 = 208745;
const MAP: u32 = 2651;
const BIT: u32 = 0;
const MOLDS: u32 = 420659;
const MIND: u32 = 426145;
const PICKAXE: u32 = 421274;
const THROW: u32 = 420696;

fn candle_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(CANDLE_KING, MAP, BIT, difficulty, None);
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

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn create_peer_copy(h: &mut Harness, difficulty: u32) -> InMap {
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
    peer_map
}

#[test]
fn encounter_coverage_candle_king_native_redundant_self_and_explicit_enemy_control() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = candle_copy(difficulty);
        let frostbolt =
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, 116, Some(boss), false);
        assert_eq!(
            frostbolt,
            Err(crate::spell_cast::CastFailure::InvalidTarget),
            "real explicit enemy still rejects self"
        );
        let result =
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, MOLDS, Some(boss), false);
        assert_eq!(
            result,
            Ok(()),
            "TC clears supplied self because native22/7 needs no explicit unit"
        );
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(boss)
            .unwrap();
        assert_eq!(cast.spell_id, MOLDS);
        assert_eq!(cast.duration, 1.5);
        h.advance(1.6);
        assert_eq!(completed_casts(&h, boss, MOLDS), 1);
    }
}

#[test]
fn encounter_coverage_candle_king_first_molds_normal_and_projected205_tc_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = candle_copy(difficulty);
        h.engage(boss);
        h.advance(5.8);
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
            .expect("TC first Molds6s must enter ordinary native cast");
        assert_eq!(cast.spell_id, MOLDS, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.5, "native70448 CastingTimeIndex243");
    }
}

#[test]
fn encounter_coverage_candle_king_ongoing_cooldowns_fear_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = candle_copy(difficulty);
        let peer_map = create_peer_copy(&mut h, difficulty);
        assert_ne!(map, peer_map);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(13.05);
        assert!(
            h.has_aura(h.player(), MIND),
            "native70448 fear from actual TC request"
        );
        assert!(
            !h.has_aura(h.players[1].0, MIND),
            "coincident other-copy actor cannot be feared"
        );
        h.advance(76.95);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let lanes = [
            (MOLDS, &[6.0, 44.3, 82.8][..]),
            (MIND, &[10.5, 31.1, 51.7, 72.3][..]),
            (PICKAXE, &[13.2, 30.2, 50.8, 67.8, 89.3][..]),
            (THROW, &[19.3, 36.3, 55.8, 74.8][..]),
        ];
        for (spell, times) in lanes {
            let lane: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(
                lane.len(),
                times.len(),
                "TC cooldown deferral {spell}: {lane:?}"
            );
            for (attempt, expected) in lane.iter().zip(times) {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "actual native occupancy {attempt:?}, expected{expected}"
                );
                assert!(
                    attempt.result.is_ok(),
                    "TC checks remaining cooldown before casting: {attempt:?}"
                );
            }
            assert_eq!(
                completed_casts(&h, boss, spell),
                times.len(),
                "native completion{spell}"
            );
        }
        assert_eq!(
            attempts.len(),
            17,
            "four native lanes plus source Darklight"
        );
        assert_eq!(
            completed_casts(&h, boss, 426152),
            1,
            "TC engagement Darklight"
        );
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
                .any(|end| end.encounter_id == 2787 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(35.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
fn encounter_coverage_candle_king_evade_rearms_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = candle_copy(difficulty);
        h.engage(boss);
        h.advance(20.0);
        assert_eq!(completed_casts(&h, boss, MOLDS), 1);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(5.0);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        let casts = h.inbox(0).spell_gos.len();
        h.advance(35.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "evade retires old lanes");
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(5.8);
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
            MOLDS
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
