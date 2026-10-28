//! Lua2777a9d initial phases only; explicitly projected Normal1/Follower205.
use super::*;

const MAP: u32 = 2519;
const CHARGATH: u32 = 189340;
const SARGHA: u32 = 189901;
const WAVE: u32 = 373742;
const STRIKE: u32 = 373733;
const SPEAR: u32 = 373424;
const KILN: u32 = 377204;
const GOLD: u32 = 377017;
const EMBER: u32 = 377473;

struct InitialPhase {
    entry: u32,
    bit: u32,
    encounter: u32,
    first_spell: u32,
    first_at: f32,
    counts: &'static [(u32, usize)],
}

const INITIAL_PHASES: &[InitialPhase] = &[
    InitialPhase {
        entry: CHARGATH,
        bit: 3,
        encounter: 2613,
        first_spell: WAVE,
        first_at: 5.0,
        counts: &[(WAVE, 2), (STRIKE, 1), (SPEAR, 1)],
    },
    InitialPhase {
        entry: SARGHA,
        bit: 1,
        encounter: 2611,
        first_spell: KILN,
        first_at: 7.1,
        counts: &[(KILN, 2), (GOLD, 1), (EMBER, 1)],
    },
];

fn projected_initial_boss(profile: &InitialPhase, difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) =
        bound_boss_at_difficulty(profile.entry, MAP, profile.bit, difficulty, None);
    let maps = h.app.world().resource::<MapStore>().clone();
    let other_map = h
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
        .expect("actual separate Neltharus copy")
        .map;
    assert_ne!(map, other_map);
    let peer = h.players[1].0;
    h.app.world_mut().entity_mut(peer).insert(other_map);
    let position = *h.app.world().get::<Position>(boss).unwrap();
    for (player, _, _) in &h.players {
        *h.app.world_mut().get_mut::<Position>(*player).unwrap() = Position {
            x: position.x + 3.0,
            ..position
        };
    }
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn arrival_cast_count(world: &World, caster: Entity, spell: u32) -> usize {
    world
        .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(actor, id, _)| *actor == caster && *id == spell)
        .count()
}

#[test]
#[ignore = "RED before generic effect41 arrival spell port"]
fn encounter_coverage_neltharus_initial_jump_arrival_chargath_midair_then_child() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_initial_boss(&INITIAL_PHASES[0], difficulty);
        h.app
            .insert_resource(crate::spell_cast::test_harness::ObservedTriggeredCasts::default());
        let player = h.player();
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        let destination = Position {
            x: origin.x + 20.0,
            ..origin
        };
        *h.app.world_mut().get_mut::<Position>(player).unwrap() = destination;
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, STRIKE, Some(player), false)
            .expect("actual native Chargath jump parent");
        h.advance(2.1);
        let during = *h.app.world().get::<Position>(boss).unwrap();
        assert!(
            during.x > origin.x && during.x < destination.x && during.y > origin.y,
            "native41 moves through a real arc, not a launch teleport: {during:?}"
        );
        assert_eq!(
            arrival_cast_count(h.app.world(), boss, 373735),
            0,
            "no child before physical arrival"
        );
        for _ in 0..40 {
            if arrival_cast_count(h.app.world(), boss, 373735) > 0 {
                break;
            }
            h.advance(0.05);
        }
        assert_eq!(arrival_cast_count(h.app.world(), boss, 373735), 1);
        assert_eq!(
            *h.app.world().get::<Position>(boss).unwrap(),
            destination,
            "child launches at the real landing endpoint"
        );
        assert!(
            h.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
                .0
                .contains(&(boss, 373735, Some(player))),
            "TC effect41 retains the arrival unit GUID"
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(373735)
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "real native arrival damage"
        );
        h.advance(0.1);
        assert_eq!(
            arrival_cast_count(h.app.world(), boss, 373735),
            1,
            "one arrival cast, not every finished frame"
        );
    }
}

fn destination_jump_fixture() -> crate::spell_cast::test_harness::Harness {
    let mut h =
        crate::spell_cast::test_harness::harness(Vec::new(), &[92832, 440206, 4336], 6).unwrap();
    h.app.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app
        .insert_resource(crate::spell_cast::test_harness::ObservedTriggeredCasts::default());
    h.app.add_systems(
        FixedUpdate,
        (
            crate::scripted_ai::jump_movement::release_finished_jumps,
            crate::scripted_ai::jump_movement::advance_jumps,
        )
            .chain(),
    );
    h.advance(3);
    h
}

#[test]
#[ignore = "RED before generic effect42 arrival spell port"]
fn encounter_coverage_neltharus_initial_jump_arrival_native_destination_trigger() {
    let mut h = destination_jump_fixture();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let destination = Vec3::new(origin.x + 21.0, origin.y, origin.z);
    crate::spell_cast::cast_triggered_at_destination(
        h.app.world_mut(),
        h.caster,
        92832,
        destination,
        0,
    );
    h.advance(1);
    let during = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!(
        during.x > origin.x && during.x < destination.x && during.y > origin.y,
        "native42 trigger option retains real jump movement"
    );
    assert_eq!(arrival_cast_count(h.app.world(), h.caster, 440206), 0);
    h.advance(40);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        Position {
            x: destination.x,
            y: destination.y,
            z: destination.z
        }
    );
    assert_eq!(arrival_cast_count(h.app.world(), h.caster, 440206), 1);
    assert!(
        h.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
            .0
            .contains(&(h.caster, 440206, None)),
        "TC effect42 has no arrival unit GUID"
    );
    h.advance(10);
    assert_eq!(arrival_cast_count(h.app.world(), h.caster, 440206), 1);
}

#[test]
#[ignore = "Native no-trigger jump control for generic arrival port"]
fn encounter_coverage_neltharus_initial_jump_arrival_native_no_trigger_control() {
    let mut h = destination_jump_fixture();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let destination = Vec3::new(origin.x + 21.0, origin.y, origin.z);
    crate::spell_cast::cast_triggered_at_destination(
        h.app.world_mut(),
        h.caster,
        4336,
        destination,
        0,
    );
    h.advance(1);
    let during = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!(during.x > origin.x && during.x < destination.x && during.y > origin.y);
    h.advance(40);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        Position {
            x: destination.x,
            y: destination.y,
            z: destination.z
        }
    );
    assert!(
        h.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
            .0
            .iter()
            .all(|(_, spell, _)| *spell == 4336),
        "native zero TriggerSpell invents no arrival cast"
    );
}

fn assert_native_primary_admission(profile: &InitialPhase) {
    for difficulty in [1, 205] {
        for &(spell, _) in profile.counts {
            let (mut h, boss, _) = projected_initial_boss(profile, difficulty);
            h.engage(boss);
            let target = if matches!(spell, STRIKE | GOLD) {
                Some(h.player())
            } else {
                None
            };
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, target, false)
                .unwrap_or_else(|error| {
                    panic!("native {spell} primary projected{difficulty}: {error:?}")
                });
            h.advance(6.0);
            assert!(
                completed_casts(&h, boss, spell) >= 1,
                "actual native completion {spell} projected{difficulty}"
            );
            assert!(
                h.inbox(1)
                    .spell_gos
                    .iter()
                    .all(|go| go.caster != boss.to_bits())
            );
        }
    }
}

#[test]
#[ignore = "Neltharus native Chargath admission qualification before AI"]
fn encounter_coverage_neltharus_initial_chargath_projected_native_admission() {
    assert_native_primary_admission(&INITIAL_PHASES[0]);
}

#[test]
#[ignore = "Neltharus native Sargha admission qualification before AI"]
fn encounter_coverage_neltharus_initial_sargha_projected_native_admission() {
    assert_native_primary_admission(&INITIAL_PHASES[1]);
}

fn assert_native_child_damage(profile: &InitialPhase, spell: u32, child: u32) {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_initial_boss(profile, difficulty);
        h.engage(boss);
        let player = h.player();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, Some(player), false)
            .expect("native parent with native victim target");
        h.advance(6.0);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(child)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual {spell}→{child} damage projected{difficulty}"
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
#[ignore = "Neltharus native Dragon Strike child qualification before AI"]
fn encounter_coverage_neltharus_initial_chargath_projected_native_child_damage() {
    assert_native_child_damage(&INITIAL_PHASES[0], STRIKE, 373735);
}

#[test]
#[ignore = "Neltharus native Molten Gold child qualification before AI"]
fn encounter_coverage_neltharus_initial_sargha_projected_native_child_damage() {
    assert_native_child_damage(&INITIAL_PHASES[1], GOLD, 377018);
}

fn assert_source_first_clock(profile: &InitialPhase) {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_initial_boss(profile, difficulty);
        h.engage(boss);
        h.advance(profile.first_at - 0.2);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.4);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(profile.first_spell),
            "source first cast {} projected{difficulty}",
            profile.entry
        );
    }
}

#[test]
#[ignore = "RED before Chargath initial-phase native scripting"]
fn encounter_coverage_neltharus_initial_chargath_projected_first_clock() {
    assert_source_first_clock(&INITIAL_PHASES[0]);
}

#[test]
#[ignore = "RED before Sargha initial-phase native scripting"]
fn encounter_coverage_neltharus_initial_sargha_projected_first_clock() {
    assert_source_first_clock(&INITIAL_PHASES[1]);
}

#[test]
#[ignore = "RED before Neltharus initial-phase native scripting"]
fn encounter_coverage_neltharus_initial_projected_finite_lanes_and_controller_gaps() {
    for profile in INITIAL_PHASES {
        for difficulty in [1, 205] {
            let (mut h, boss, _) = projected_initial_boss(profile, difficulty);
            h.engage(boss);
            h.advance(90.0);
            assert!(h.health(h.player()).current > 0.0, "actual victim alive");
            for &(spell, expected) in profile.counts {
                assert_eq!(
                    completed_casts(&h, boss, spell),
                    expected,
                    "source finite lane {spell} projected{difficulty}"
                );
            }
            for spell in [375056, 376780, 377014] {
                assert_eq!(
                    completed_casts(&h, boss, spell),
                    0,
                    "no fabricated energy, shield or Backdraft controller"
                );
            }
        }
    }
}

#[test]
#[ignore = "RED before Neltharus initial-phase native scripting"]
fn encounter_coverage_neltharus_initial_projected_generic_credit_copy_and_corpse() {
    for profile in INITIAL_PHASES {
        for difficulty in [1, 205] {
            let (mut h, boss, map) = projected_initial_boss(profile, difficulty);
            h.engage(boss);
            h.advance(11.0);
            assert_eq!(completed_casts(&h, boss, profile.first_spell), 1);
            assert!(
                h.inbox(0)
                    .starts
                    .iter()
                    .any(|event| event.encounter_id == profile.encounter
                        && event.difficulty_id == difficulty)
            );
            assert!(h.inbox(1).starts.is_empty());
            assert!(
                h.inbox(1)
                    .spell_gos
                    .iter()
                    .all(|go| go.caster != boss.to_bits())
            );
            h.hit_pct(boss, 200.0);
            h.advance(0.1);
            assert_eq!(
                instance_script::get_boss_state(h.app.world_mut(), map, profile.bit),
                Some(EncounterState::Done)
            );
            assert_eq!(
                h.app
                    .world()
                    .resource::<crate::maps::InstanceManager>()
                    .completed_encounters(map),
                1 << profile.bit
            );
            assert!(
                h.inbox(0)
                    .ends
                    .iter()
                    .any(|event| event.encounter_id == profile.encounter && event.success)
            );
            assert!(h.inbox(1).ends.is_empty());
            let before = h.inbox(0).spell_gos.len();
            h.advance(50.0);
            assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
        }
    }
}

#[test]
#[ignore = "RED before Neltharus initial-phase native scripting"]
fn encounter_coverage_neltharus_initial_projected_native_home_reset_rearms_initial_phase() {
    for profile in INITIAL_PHASES {
        for difficulty in [1, 205] {
            let (mut h, boss, map) = projected_initial_boss(profile, difficulty);
            h.engage(boss);
            h.advance(11.0);
            assert_eq!(completed_casts(&h, boss, profile.first_spell), 1);
            crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
            assert_eq!(
                instance_script::get_boss_state(h.app.world_mut(), map, profile.bit),
                Some(EncounterState::NotStarted)
            );
            let player = h.player();
            h.app.world_mut().get_mut::<Position>(player).unwrap().x += 500.0;
            for _ in 0..100 {
                if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
                    != Some(&crate::creature_ai::CreatureState::Returning)
                {
                    break;
                }
                h.advance(0.1);
            }
            assert_ne!(
                h.app.world().get::<crate::creature_ai::CreatureState>(boss),
                Some(&crate::creature_ai::CreatureState::Returning)
            );
            let position = *h.app.world().get::<Position>(boss).unwrap();
            *h.app.world_mut().get_mut::<Position>(player).unwrap() = Position {
                x: position.x + 3.0,
                ..position
            };
            h.engage(boss);
            h.advance(profile.first_at - 0.2);
            assert!(h.app.world().get::<CastState>(boss).is_none());
            h.advance(0.4);
            assert_eq!(
                h.app
                    .world()
                    .get::<CastState>(boss)
                    .map(|cast| cast.spell_id),
                Some(profile.first_spell)
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
}
