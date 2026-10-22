//! LittleWigs2777a9d BenkBuzzbee.lua; source218002, native70448.
use super::*;

const BENK: u32 = 218002;
const MAP: u32 = 2661;
const BIT: u32 = 3;
const SNACK: u32 = 438025;
const HONEY: u32 = 440134;
const WING: u32 = 439524;

fn benk_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(BENK, MAP, BIT, difficulty, None);
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

fn assert_native_wing_channel(h: &mut Harness, boss: Entity, map: InMap) {
    let channel = h
        .app
        .world()
        .get::<shared::casting::CastState>(boss)
        .expect("native Wing channel after actual1.5s cast");
    assert_eq!(channel.spell_id, WING);
    assert_eq!(channel.cast_type, shared::casting::CastType::Channel);
    assert_eq!(
        channel.duration, 2.0,
        "native Attributes1 0x40/Duration2000ms"
    );
    assert!(
        h.has_aura(boss, WING),
        "native caster dummy aura, no fake damage child"
    );
    let world = h.app.world_mut();
    let fields: Vec<_> = world
        .query::<(&crate::spell_cast::area_trigger::SpellAreaTrigger, &InMap)>()
        .iter(world)
        .filter(|(field, _)| field.caster == boss && field.spell_id == WING)
        .map(|(field, field_map)| (*field_map, field.trigger_spell))
        .collect();
    assert_eq!(
        fields,
        vec![(map, 0)],
        "actual native area lifecycle; absent trigger stays absent"
    );
}

#[test]
fn encounter_coverage_benk_first_snack_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = benk_copy(difficulty);
        h.engage(boss);
        h.advance(2.8);
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
            .expect("source Snack Time3s ordinary native cast");
        assert_eq!(cast.spell_id, SNACK, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.0, "actual70448 native cast time");
    }
}

#[test]
fn encounter_coverage_benk_ongoing_native_aura_channel_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = benk_copy(difficulty);
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
        h.advance(12.2);
        assert!(
            h.has_aura(h.player(), HONEY),
            "actual targeted native5s Honey aura"
        );
        assert!(
            !h.has_aura(peer, HONEY),
            "coincident peer in another copy is excluded"
        );
        h.advance(11.4);
        assert_native_wing_channel(&mut h, boss, map);
        assert!(
            !h.has_aura(h.player(), HONEY),
            "first native Honey duration expired"
        );
        h.advance(66.4);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let lanes = [
            (SNACK, &[3.0, 36.0, 73.0][..]),
            (HONEY, &[10.0, 25.5, 39.5, 53.5, 67.5, 81.5][..]),
            (WING, &[22.0, 45.0, 69.5][..]),
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
                .any(|end| end.encounter_id == 2931 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
fn encounter_coverage_benk_evade_rearms_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = benk_copy(difficulty);
        h.engage(boss);
        h.advance(40.0);
        assert!(count_cast_receipts(&h, boss, SNACK) > 0);
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
        let before = count_cast_receipts(&h, boss, SNACK);
        h.engage(boss);
        h.advance(2.8);
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
            SNACK
        );
        h.advance(2.1);
        assert_eq!(count_cast_receipts(&h, boss, SNACK), before + 1);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
