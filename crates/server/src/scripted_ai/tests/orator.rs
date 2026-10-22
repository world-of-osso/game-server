//! TC a352b1fa CityOfThreads/boss_orator_krix_vizk.cpp; native70448 actor216619.
//! Follower205 explicitly projects the authored Normal placement, without
//! publishing invented205 spawn/stat rows.
use super::*;
use shared::components::PowerType;

const ORATOR: u32 = 216619;
const MAP: u32 = 2669;
const BIT: u32 = 1;
const SUBJUGATE: u32 = 434722;

fn orator_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ORATOR, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    assert!(
        h.app.world().get::<Mana>(boss).is_none(),
        "actual sourceclass4 is Energy"
    );
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

fn energy(h: &Harness, boss: Entity) -> f32 {
    crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy)
        .expect("sourceclass4 native Energy pool")
        .0
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
#[ignore = "RED: TC Orator AI JustAppeared and combat are not ported"]
fn encounter_coverage_orator_source_appearance_and_first_subjugate_normal_projected205() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = orator_copy(difficulty);
        assert_eq!(
            energy(&h, boss),
            0.0,
            "TC JustAppeared setsEnergy0; fixture must not forge it"
        );
        assert!(h.has_aura(boss, 454689), "TC JustAppeared native Oration");
        h.engage(boss);
        h.advance(0.8);
        assert_eq!(energy(&h, boss), 0.0);
        h.advance(0.3);
        assert_eq!(energy(&h, boss), 4.0, "TC first1s grants4");
        h.advance(1.0);
        assert_eq!(energy(&h, boss), 4.0);
        h.advance(0.2);
        assert_eq!(energy(&h, boss), 8.0, "TC next1.2s");
        h.advance(0.8);
        assert_eq!(energy(&h, boss), 12.0, "TC next0.8s");
        h.advance(1.2);
        assert_eq!(energy(&h, boss), 16.0);
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
            .expect("TC Subjugate first4.5s ordinary native cast");
        assert_eq!(cast.spell_id, SUBJUGATE, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.25, "actual70448 CastingTimeIndex206");
    }
}

#[test]
#[ignore = "Qualification: native Subjugate effects, not a boss timer port"]
fn encounter_coverage_orator_native_subjugate_damage_aura_and_copy_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = orator_copy(difficulty);
        let player = h.player();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SUBJUGATE, Some(player), false)
            .expect("ordinary native Subjugate, no invented power or triggered flags");
        h.advance(1.4);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(SUBJUGATE)
                    && event.amount > 0)
        );
        assert!(
            h.has_aura(player, SUBJUGATE),
            "native aura33 belongs to the actual hit recipient"
        );
        assert!(
            !h.has_aura(h.players[1].0, SUBJUGATE),
            "coincident peer copy is isolated"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "RED: TC Orator energy and ongoing request lanes not ported"]
fn encounter_coverage_orator_ongoing_two_energy_phases_native_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = orator_copy(difficulty);
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
        h.advance(90.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let lanes = [
            (SUBJUGATE, &[4.5, 21.6, 33.6, 50.7, 65.75, 82.85][..]),
            (434808, &[9.4, 17.5, 38.6, 46.7, 67.8, 75.9][..]),
            (434829, &[28.05, 60.75][..]),
        ];
        assert_eq!(
            attempts.len(),
            15,
            "TC14 combat requests plus engagement Chains: {attempts:?}"
        );
        for (spell, times) in lanes {
            let lane: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(lane.len(), times.len(), "source lane{spell}: {lane:?}");
            for (attempt, expected) in lane.iter().zip(times) {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "TC/native occupancy {attempt:?}, expected{expected}"
                );
                assert!(
                    attempt.result.is_ok(),
                    "ordinary source admission{attempt:?}"
                );
            }
            assert_eq!(completed_casts(&h, boss, spell), times.len());
        }
        assert_eq!(
            completed_casts(&h, boss, 448560),
            0,
            "no Mythic-only Shadows in Normal/projected205"
        );
        for (spell, count) in [(SUBJUGATE, 6), (434779, 6), (434832, 8)] {
            let hits = h
                .inbox(0)
                .log
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Damage
                        && event.source == Some(boss.to_bits())
                        && event.target == Some(h.player().to_bits())
                        && event.spell_id == Some(spell)
                        && event.amount > 0
                })
                .count();
            assert_eq!(
                hits, count,
                "actual native damage{spell}, difficulty{difficulty}"
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
                .any(|end| end.encounter_id == 2907 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = [SUBJUGATE, 434808, 434829].map(|spell| completed_casts(&h, boss, spell));
        h.advance(35.0);
        assert_eq!(
            [SUBJUGATE, 434808, 434829].map(|spell| completed_casts(&h, boss, spell)),
            casts,
            "corpse parent clocks stay silent"
        );
    }
}

#[test]
#[ignore = "RED: TC Orator reset and source counters not ported"]
fn encounter_coverage_orator_generic_evade_rearms_clock_without_forging_respawn_energy() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = orator_copy(difficulty);
        h.engage(boss);
        h.advance(20.0);
        assert_eq!(completed_casts(&h, boss, SUBJUGATE), 1);
        let before = energy(&h, boss);
        assert_eq!(
            before, 76.0,
            "source19 grants before20s, no automatic regeneration"
        );
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
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            casts,
            "generic evade retires clocks"
        );
        assert_eq!(
            energy(&h, boss),
            before,
            "TC despawn/JustAppeared is not generic same-actor reset; do not forge Energy0"
        );
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(4.3);
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
            SUBJUGATE
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
