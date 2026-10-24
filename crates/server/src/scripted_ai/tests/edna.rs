//! TC a352b1fa Stonevault: template-inherited StringId, real deaths and intro.
//! Normal source placements;205 explicitly projects identical authored actors.
use super::*;

#[path = "edna_seismic_shield.rs"]
mod seismic_shield;

const EDNA: u32 = 210108;
const GOLEM: u32 = 210109;
const MAP: u32 = 2652;
const INTRO_STATE: u32 = 6;
const IMMUNE_TO_PC: u32 = 256;
const ANCHOR: u32 = 45313;
const COMBAT_POSITION: Position = Position {
    x: 1.94097,
    y: 361.66537,
    z: -0.512153,
};

fn source_copy(difficulty: u32) -> (Harness, Entity, InMap, Vec<Entity>) {
    let (mut h, boss, map) = bound_boss_at_difficulty(EDNA, MAP, 0, difficulty, None);
    let connection =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut spawns: Vec<_> = crate::world_data::load_spawns(&connection)
        .into_iter()
        .filter(|spawn| spawn.template_entry == GOLEM && u32::from(spawn.map) == MAP)
        .collect();
    spawns.sort_by_key(|spawn| spawn.guid);
    assert_eq!(
        spawns.iter().map(|spawn| spawn.guid).collect::<Vec<_>>(),
        [30002119, 30002123, 30002124, 30002127, 30002177]
    );
    let golems = spawns
        .into_iter()
        .map(|mut spawn| {
            assert_eq!(spawn.spawn_difficulties, [1, 2, 8, 23]);
            spawn.spawn_difficulties = vec![difficulty];
            spawn_prepared_actor(&mut h.app, spawn, map, difficulty).0
        })
        .collect::<Vec<_>>();
    for &golem in &golems {
        let ids = h
            .app
            .world()
            .get::<crate::zones::CreatureStringIds>(golem)
            .unwrap();
        assert_eq!(ids.template.as_deref(), Some("edna_intro_trash"));
        assert!(ids.spawn.is_none());
        assert!(ids.has("edna_intro_trash"));
    }
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.advance(0.05);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 320);
    (h, boss, map, golems)
}

fn unlock(h: &mut Harness, boss: Entity, map: InMap, golems: &[Entity]) {
    for &golem in golems {
        super::herald_intro::kill_actor(h, golem);
    }
    for _ in 0..500 {
        h.advance(0.05);
        if intro_state(h, map) == 3 {
            break;
        }
    }
    assert_eq!(
        intro_state(h, map),
        3,
        "authentic five-death/movement unlock"
    );
    assert!(!immune(h, boss));
    let mut near = *h.app.world().get::<Position>(boss).unwrap();
    near.x += 3.0;
    let player = h.player();
    let peer = h.players[1].0;
    h.app.world_mut().entity_mut(player).insert(near);
    h.app.world_mut().entity_mut(peer).insert(near);
}

fn intro_state(h: &mut Harness, map: InMap) -> u32 {
    instance_script::get_data(h.app.world_mut(), map, INTRO_STATE)
}

fn immune(h: &Harness, actor: Entity) -> bool {
    h.app.world().get::<UnitFlags>(actor).unwrap().0 & IMMUNE_TO_PC != 0
}

#[test]
fn encounter_coverage_edna_five_inherited_stringid_deaths_force_walk_arrival_unlock() {
    for difficulty in [1, 205] {
        let (mut h, boss, map, golems) = source_copy(difficulty);
        let clock = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin: clock,
                entries: Vec::new(),
            });
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        for &golem in &golems[..4] {
            super::herald_intro::kill_actor(&mut h, golem);
        }
        h.advance(4.0);
        assert_eq!(intro_state(&mut h, map), 0);
        assert_eq!(h.app.world().get::<Position>(boss), Some(&origin));
        assert!(immune(&h, boss));
        super::herald_intro::kill_actor(&mut h, golems[4]);
        assert_eq!(
            intro_state(&mut h, map),
            1,
            "fifth real death starts instance intro"
        );
        h.advance(2.8);
        assert_eq!(h.app.world().get::<Position>(boss), Some(&origin));
        assert!(immune(&h, boss));
        h.advance(0.3);
        assert!(h.app.world().get::<movement::Walking>(boss).is_some());
        assert!(h.app.world().get::<movement::PointMovement>(boss).is_some());
        assert!(
            !h.has_aura(boss, 447230),
            "source removes Skardyn spawn aura before walking"
        );
        assert_eq!(intro_state(&mut h, map), 1);
        for _ in 0..500 {
            h.advance(0.05);
            if h.app.world().get::<movement::PointMovement>(boss).is_none() {
                break;
            }
        }
        assert_eq!(h.app.world().get::<Position>(boss), Some(&COMBAT_POSITION));
        assert!(
            immune(&h, boss),
            "MovementInform schedules +1s, not immediate unlock"
        );
        assert_eq!(intro_state(&mut h, map), 1);
        h.advance(0.8);
        assert!(immune(&h, boss));
        h.advance(0.3);
        assert_eq!(
            intro_state(&mut h, map),
            3,
            "TC DONE from authored arrival callback"
        );
        assert!(!immune(&h, boss));
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
        assert_eq!(
            crate::spell_cast::powers::power_value(
                h.app.world(),
                boss,
                shared::components::PowerType::Energy,
            )
            .unwrap()
            .0,
            4.0,
            "combat ingress refresh must not replay class-power initialization"
        );
        let home = h
            .app
            .world()
            .get::<crate::creature_ai::HomePosition>(boss)
            .expect("native Anchor Here updates home, not original respawn data");
        assert_eq!(home.position, COMBAT_POSITION);
        assert_eq!(
            home.orientation,
            h.app.world().get::<Rotation>(boss).unwrap().y
        );
        // Native bool FULL_MASK does not emit ordinary SpellGo. Qualify its
        // accepted request together with the real home-position effect above.
        let anchors: Vec<_> = h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries
            .iter()
            .filter(|attempt| attempt.spell == ANCHOR)
            .collect();
        assert_eq!(anchors.len(), 1);
        assert!(
            anchors[0].result.is_ok(),
            "native45313 admission after arrival+1s"
        );
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::NotStarted)
        );
    }
}

#[test]
fn encounter_coverage_edna_death_counter_is_copy_owned_and_idle_time_does_not_unlock() {
    let (mut h, boss, map, golems) = source_copy(1);
    let other = allocate_peer_copy(&mut h, 1);
    assert_ne!(other, map);
    // Existing fixture peer belongs to this distinct copy. Moving one authentic
    // golem there lets the real lethal boundary prove copy ownership.
    h.app.world_mut().entity_mut(golems[0]).insert(other);
    super::herald_intro::kill_actor(&mut h, golems[0]);
    for &golem in &golems[1..] {
        super::herald_intro::kill_actor(&mut h, golem);
    }
    h.advance(25.0);
    assert_eq!(intro_state(&mut h, map), 0);
    assert_eq!(intro_state(&mut h, other), 0);
    assert!(immune(&h, boss));
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::HomePosition>(boss)
            .is_none()
    );
    assert!(h.app.world().get::<movement::PointMovement>(boss).is_none());
}

#[test]
fn encounter_coverage_edna_native_anchor_sets_home_position_without_unlock() {
    let (mut h, boss, map, _) = source_copy(1);
    h.app.world_mut().entity_mut(boss).insert(COMBAT_POSITION);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, ANCHOR, Some(boss), false)
        .expect("ordinary native45313 script effect");
    let home = h
        .app
        .world()
        .get::<crate::creature_ai::HomePosition>(boss)
        .expect("TC spell_gen_anchor_here effect0 sets current home position");
    assert_eq!(home.position, COMBAT_POSITION);
    assert_eq!(
        home.orientation,
        h.app.world().get::<Rotation>(boss).unwrap().y
    );
    assert!(
        immune(&h, boss),
        "native anchor alone does not remove PC immunity"
    );
    assert_eq!(intro_state(&mut h, map), 0);
}

#[test]
fn encounter_coverage_edna_appearance_sets_source_energy_four_without_unlock() {
    for difficulty in [1, 205] {
        let (h, boss, _, _) = source_copy(difficulty);
        let energy = crate::spell_cast::powers::power_value(
            h.app.world(),
            boss,
            shared::components::PowerType::Energy,
        )
        .unwrap()
        .0;
        assert_eq!(energy, 4.0, "TC JustAppeared; no fixture power grant");
        assert!(immune(&h, boss));
    }
}

#[test]
fn encounter_coverage_edna_native_seismic_smash_damage_and_copy_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _, _) = source_copy(difficulty);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        let player = h.player();
        let peer = h.players[1].0;
        h.app.world_mut().entity_mut(player).insert(near);
        h.app.world_mut().entity_mut(peer).insert(near);
        // Native melee spells may miss. Scope only the real cast boundary so
        // this positive damage qualification is independent of setup RNG.
        let _rng = crate::random::seed_scope(0);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 424888, Some(player), false)
            .expect("ordinary sourced victim cast, no immunity edits or bypass flags");
        assert_eq!(h.app.world().get::<CastState>(boss).unwrap().duration, 4.0);
        h.advance(4.2);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(424888)
                    && event.amount > 0),
            "native Seismic Smash damage difficulty{difficulty}; logs={:?}; gos={:?}",
            h.inbox(0).log,
            h.inbox(0).spell_gos
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        assert!(
            immune(&h, boss),
            "outgoing qualification never strips source PC immunity"
        );
    }
}

#[test]
fn encounter_coverage_edna_first_spike_after_authentic_unlock_normal_projected205() {
    for difficulty in [1, 205] {
        let (mut h, boss, map, golems) = source_copy(difficulty);
        unlock(&mut h, boss, map, &golems);
        h.engage(boss);
        h.advance(7.9);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<CastState>(boss)
            .expect("TC first Volatile Spike8.1s native admission");
        assert_eq!(cast.spell_id, 424903);
        assert_eq!(cast.duration, 2.5, "native70448 CastingTimeIndex20");
    }
}

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn allocate_peer_copy(h: &mut Harness, difficulty: u32) -> InMap {
    let maps = h.app.world().resource::<MapStore>().clone();
    let copy = h
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
        .insert((copy, crate::persistence::ClientId(2002)));
    copy
}

#[test]
fn encounter_coverage_edna_ongoing_native_occupancy_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map, golems) = source_copy(difficulty);
        unlock(&mut h, boss, map, &golems);
        let peer = allocate_peer_copy(&mut h, difficulty);
        assert_ne!(peer, map);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        // Both required Seismic damage receipts remain genuine native hits;
        // replay their hit-table draws independently of intro/setup randomness.
        let _rng = crate::random::seed_scope(0);
        h.engage(boss);
        h.advance(60.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        for (spell, times) in [
            (424903, &[8.1, 22.7, 37.3, 51.9][..]),
            (452738, &[11.8, 25.2, 36.1, 47.0, 57.9][..]),
            (424888, &[15.4, 39.8][..]),
        ] {
            let lane: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert_eq!(lane.len(), times.len(), "source lane{spell}: {attempts:?}");
            for (attempt, expected) in lane.into_iter().zip(times) {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "source/native occupancy {attempt:?}, expected{expected}; all{attempts:?}"
                );
                assert!(
                    attempt.result.is_ok(),
                    "source request admission{attempt:?}"
                );
            }
            assert_eq!(completed_casts(&h, boss, spell), times.len());
        }
        let player = h.player();
        assert_eq!(
            h.inbox(0)
                .log
                .iter()
                .filter(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(424888)
                    && event.amount > 0)
                .count(),
            2
        );
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        crate::combat::kill_unit(h.app.world_mut(), player, boss);
        h.advance(0.1);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(peer),
            0
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == 2854 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = [424903, 452738, 424888].map(|spell| completed_casts(&h, boss, spell));
        h.advance(30.0);
        assert_eq!(
            [424903, 452738, 424888].map(|spell| completed_casts(&h, boss, spell)),
            before
        );
    }
}

#[test]
fn encounter_coverage_edna_generic_evade_keeps_intro_done_and_rearms_parent_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, map, golems) = source_copy(difficulty);
        unlock(&mut h, boss, map, &golems);
        h.engage(boss);
        h.advance(11.0);
        assert_eq!(completed_casts(&h, boss, 424903), 1);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(30.0);
        assert_eq!(
            intro_state(&mut h, map),
            3,
            "actor reset never owns copy intro state"
        );
        assert!(!immune(&h, boss));
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::NotStarted)
        );
        let before = completed_casts(&h, boss, 424903);
        assert_eq!(before, 1);
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        h.engage(boss);
        h.advance(7.9);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app.world().get::<CastState>(boss).unwrap().spell_id,
            424903
        );
        assert_eq!(
            completed_casts(&h, boss, 424903),
            before,
            "new native cast has not completed"
        );
    }
}
