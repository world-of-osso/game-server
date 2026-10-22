//! LittleWigs2777a9d AraKara/Anubzekt.lua; actual source215405, native70448.
use super::*;

const ANUBZEKT: u32 = 215405;
const MAP: u32 = 2660;
const BIT: u32 = 1;
const IMPALE: u32 = 435012;
const INFESTATION: u32 = 433740;
const BURROW: u32 = 439506;
const EYE: u32 = 433766;

fn anubzekt_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ANUBZEKT, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let mut near = *h.app.world().get::<Position>(boss).unwrap();
    near.x += 3.0;
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(near);
    let peer = h.players[1].0;
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

#[test]
#[ignore = "native RED: source class4 Energy pool missing at ordinary spawn"]
fn encounter_coverage_creature_class4_native_energy_spawn_spend_and_regeneration() {
    use shared::components::PowerType;
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        assert!(
            !h.app
                .world()
                .get::<CombatStatus>(boss)
                .is_some_and(|status| status.0),
            "source actor stays idle until explicitly engaged"
        );
        assert!(
            h.app.world().get::<Mana>(boss).is_none(),
            "source class4 is not Mana"
        );
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((100.0, 100.0)),
            "native ChrClasses4.DisplayPower3 and70448 NPC Energy initialization"
        );
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, EYE, Some(boss), false)
            .expect("ordinary source100Energy Eye, no scripted power grant");
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((100.0, 100.0)),
            "ordinary native cast charges power on completion, not admission"
        );
        h.advance(7.2);
        assert_eq!(completed_casts(&h, boss, EYE), 1);
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((0.0, 100.0))
        );
        h.advance(2.0);
        let energy = crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy)
            .unwrap()
            .0;
        assert_eq!(
            energy, 0.0,
            "actual source unit_flags2 lacks regenerate-power0x800"
        );
        assert_eq!(completed_casts(&h, boss, EYE), 1);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    }
}

#[test]
#[ignore = "native source proof with coincident peer pending"]
fn encounter_coverage_anubzekt_native_infestation_periodic_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.1);
        let before = completed_casts(&h, boss, INFESTATION);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, INFESTATION, Some(boss), false)
            .expect("ordinary native433740 area aura");
        h.advance(2.0);
        assert_eq!(completed_casts(&h, boss, INFESTATION), before + 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(INFESTATION)
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual500ms periodic damage difficulty{difficulty}"
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
#[ignore = "native source proof with coincident peer pending"]
fn encounter_coverage_anubzekt_first_impale_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        h.engage(boss);
        h.advance(4.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<CastState>(boss)
            .expect("source4.6s ordinary Impale request");
        assert_eq!(cast.spell_id, IMPALE);
        assert_eq!(
            cast.duration, 4.0,
            "actual Normal/Follower native cast time"
        );
        h.advance(4.1);
        assert_eq!(completed_casts(&h, boss, IMPALE), 1);
    }
}

#[test]
#[ignore = "native source proof with real second-copy credit pending"]
fn encounter_coverage_anubzekt_ongoing_lanes_native_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = anubzekt_copy(difficulty);
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
        assert_ne!(
            peer_map, map,
            "native manager creates a separate actual copy"
        );
        let initial_infestations = completed_casts(&h, boss, INFESTATION);
        assert_eq!(
            initial_infestations, 0,
            "no casts before native threat ingress"
        );
        assert!(!h.has_aura(h.player(), INFESTATION));
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(0.1);
        assert!(
            h.has_aura(h.player(), INFESTATION),
            "actual immediate-on-pull native aura"
        );
        h.advance(129.9);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("ANUBZEKT difficulty{difficulty} {attempts:?}");
        let lanes = [
            (
                IMPALE,
                &[
                    4.6, 13.1, 21.9, 27.7, 42.6, 51.1, 59.6, 68.1, 76.6, 85.4, 91.2, 99.7, 108.2,
                    117.0, 122.8,
                ][..],
            ),
            (
                INFESTATION,
                &[
                    0.05, 10.15, 21.1, 39.0, 47.5, 56.0, 64.5, 73.0, 84.6, 95.2, 103.7, 116.2,
                    126.8,
                ][..],
            ),
            (BURROW, &[17.1, 80.6, 112.2][..]),
            (EYE, &[31.7, 112.2][..]),
        ];
        assert_eq!(
            attempts.len(),
            33,
            "all sourced requests including immediate ingress and ordinary second-Eye rejection"
        );
        for (spell, times) in lanes {
            let lane: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(lane.len(), times.len(), "source lane{spell}: {lane:?}");
            for (index, (attempt, expected)) in lane.iter().zip(times).enumerate() {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "source countdown/native occupancy {attempt:?}, expected{expected}"
                );
                if spell == EYE && index == 1 {
                    assert!(
                        matches!(
                            attempt.result,
                            Err(crate::spell_cast::CastFailure::NotEnoughPower(
                                shared::components::PowerType::Energy
                            ))
                        ),
                        "source actor has no automatic regeneration; no second energy grant: {attempt:?}"
                    );
                } else {
                    assert!(
                        attempt.result.is_ok(),
                        "ordinary native admission {attempt:?}"
                    );
                }
            }
            let accepted = lane.iter().filter(|a| a.result.is_ok()).count();
            let initial = if spell == INFESTATION {
                initial_infestations
            } else {
                0
            };
            assert_eq!(completed_casts(&h, boss, spell), accepted + initial);
        }
        assert!(
            attempts
                .iter()
                .all(|a| [IMPALE, INFESTATION, BURROW, EYE].contains(&a.spell)),
            "no invented child forwarding or adds"
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(INFESTATION)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(h.player().to_bits()))
        );
        assert!(h.inbox(1).spell_gos.is_empty());
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
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == 2906 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(peer_map),
            0,
            "primary death cannot credit the other actual native instance"
        );
        let casts = h.inbox(0).spell_gos.len();
        h.advance(20.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
#[ignore = "native source proof with coincident peer pending"]
fn encounter_coverage_anubzekt_evade_rearms_initial_combo_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = anubzekt_copy(difficulty);
        h.engage(boss);
        h.advance(45.0);
        assert!(completed_casts(&h, boss, EYE) > 0);
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
        h.advance(40.0);
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            casts,
            "evade retires all old lanes"
        );
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        let before = completed_casts(&h, boss, IMPALE);
        let before_eye = completed_casts(&h, boss, EYE);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(4.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app.world().get::<CastState>(boss).unwrap().spell_id,
            IMPALE
        );
        h.advance(4.1);
        assert_eq!(completed_casts(&h, boss, IMPALE), before + 1);
        h.advance(26.2);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let eyes: Vec<_> = attempts.iter().filter(|a| a.spell == EYE).collect();
        assert_eq!(
            eyes.len(),
            1,
            "reset rearms actual first-Eye request without fabricating energy"
        );
        assert!((eyes[0].time - 31.7).abs() <= 0.15);
        assert!(matches!(
            eyes[0].result,
            Err(crate::spell_cast::CastFailure::NotEnoughPower(
                shared::components::PowerType::Energy
            ))
        ));
        assert_eq!(completed_casts(&h, boss, EYE), before_eye);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
