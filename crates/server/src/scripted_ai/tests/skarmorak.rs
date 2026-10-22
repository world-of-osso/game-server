//! TC a352b1fa TheStoneVault/boss_skarmorak.cpp; native70448 source210156.
//! Normal1 uses the authored actor; Follower205 explicitly projects that same
//! native placement. No205 spawn/stat row is fabricated or published.
use super::*;
use shared::components::PowerType;

const SKARMORAK: u32 = 210156;
const MAP: u32 = 2652;
const BIT: u32 = 1;
const SMASH: u32 = 422233;

fn skarmorak_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(SKARMORAK, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    assert!(
        h.app.world().get::<Mana>(boss).is_none(),
        "actual sourceclass4 is Energy, not Mana"
    );
    assert_eq!(
        energy(&h, boss),
        100.0,
        "native class4 NPC full initialization; no fixture grant"
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
#[ignore = "RED: TC Skarmorak AI is not ported"]
fn encounter_coverage_skarmorak_first_smash_normal_and_projected205_tc_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = skarmorak_copy(difficulty);
        h.engage(boss);
        h.advance(4.4);
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
            .expect("TC first Crystalline Smash4.6s must enter ordinary native cast");
        assert_eq!(cast.spell_id, SMASH, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.0, "actual70448 casting time5");
        assert_eq!(
            energy(&h, boss),
            100.0,
            "ordinary cast spends at completion only"
        );
    }
}

#[test]
#[ignore = "Qualification: native source power and damage, not a timer port"]
fn encounter_coverage_skarmorak_native_smash_damage_and_actual_energy_cost() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = skarmorak_copy(difficulty);
        let player = h.player();
        let peer = h.players[1].0;
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SMASH, Some(player), false)
            .expect("ordinary actual422233 from sourceclass4 full Energy");
        assert_eq!(energy(&h, boss), 100.0);
        h.advance(2.2);
        assert_eq!(
            energy(&h, boss),
            75.0,
            "actual native25Energy cost, no manual spend"
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(SMASH)
                    && event.amount > 0)
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        let peer_health = h.health(peer).current;
        h.advance(1.0);
        assert_eq!(
            h.health(peer).current,
            peer_health,
            "coincident peer remains isolated"
        );
        assert_eq!(
            energy(&h, boss),
            75.0,
            "source flags2=0 does not regenerate Energy"
        );
    }
}

#[test]
#[ignore = "RED: TC Skarmorak energy/Shell/Discharge clocks not ported"]
fn encounter_coverage_skarmorak_source_shell_refill_channel_rearm_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = skarmorak_copy(difficulty);
        let peer_map = create_peer_copy(&mut h, difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(33.5);
        assert_eq!(
            energy(&h, boss),
            0.0,
            "four actual25Energy native completions"
        );
        h.advance(0.3);
        assert!(
            h.has_aura(boss, 429976),
            "TC depleted-energy check starts actual Fortified energize aura"
        );
        h.advance(0.9);
        assert_eq!(energy(&h, boss), 17.0, "TC tick1 usescycle[1]");
        h.advance(1.0);
        assert_eq!(energy(&h, boss), 34.0, "TC tick2 usescycle[2]");
        h.advance(1.0);
        assert_eq!(energy(&h, boss), 50.0, "TC tick3 usescycle[0]");
        h.advance(3.0);
        assert_eq!(
            energy(&h, boss),
            100.0,
            "six sourced ticks, not unsourced regeneration"
        );
        h.advance(140.3);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        for (spell, expected) in [
            (SMASH, &[4.6, 21.6, 171.65][..]),
            (423538, &[10.5, 29.9, 177.75][..]),
            (423324, &[39.65][..]),
        ] {
            let lane: Vec<_> = attempts
                .iter()
                .filter(|entry| entry.spell == spell)
                .collect();
            assert_eq!(lane.len(), expected.len(), "TC lane{spell}: {attempts:?}");
            for (entry, time) in lane.iter().zip(expected) {
                assert!(
                    (entry.time - time).abs() <= 0.15,
                    "native occupancy {entry:?}, expected{time}; all{attempts:?}"
                );
                assert!(entry.result.is_ok(), "ordinary sourced admission{entry:?}");
            }
            assert_eq!(completed_casts(&h, boss, spell), expected.len());
        }
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(h.player().to_bits())
                    && event.spell_id == Some(423327)
                    && event.amount > 0),
            "native VoidDischarge periodic damage"
        );
        assert!(
            h.inbox(1).spell_gos.is_empty(),
            "coincident different-copy observer cannot receive boss spells"
        );
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
                .any(|end| end.encounter_id == 2880 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = [SMASH, 423538, 423324].map(|spell| completed_casts(&h, boss, spell));
        h.advance(30.0);
        assert_eq!(
            [SMASH, 423538, 423324].map(|spell| completed_casts(&h, boss, spell)),
            casts
        );
    }
}

#[test]
#[ignore = "RED: TC Skarmorak reset clocks not ported"]
fn encounter_coverage_skarmorak_generic_evade_rearms_without_fabricated_energy() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = skarmorak_copy(difficulty);
        h.engage(boss);
        h.advance(15.0);
        assert_eq!(energy(&h, boss), 50.0);
        assert_eq!(completed_casts(&h, boss, SMASH), 1);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(5.0);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        let count = h.inbox(0).spell_gos.len();
        h.advance(35.0);
        assert_eq!(h.inbox(0).spell_gos.len(), count);
        assert_eq!(
            energy(&h, boss),
            50.0,
            "generic sameactor reset must not forge TC despawn/reappearance"
        );
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(4.4);
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
            SMASH
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
