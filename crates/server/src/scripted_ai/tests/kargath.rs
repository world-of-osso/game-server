//! TC a352b1fa boss_warchief_kargath_bladefist.cpp; native70448/map540 bit3.
use super::*;

const KARGATH: u32 = 16808;
const SHATTERED_HALLS: u32 = 540;
const KARGATH_BIT: u32 = 3;

fn kargath_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, _, map) = encounter_fixture(
        KARGATH,
        SHATTERED_HALLS,
        KARGATH_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let boss = h.creatures_of(KARGATH)[0];
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&SHATTERED_HALLS].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(SHATTERED_HALLS, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn stand_near_kargath(h: &mut Harness, boss: Entity) {
    // HARNESS SETUP: isolated entrant approaches only after the source copy is ready.
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
}

fn kargath_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&KARGATH][&group];
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && texts
                    .iter()
                    .any(|text| line.content == text.male_text || line.content == text.female_text)
        })
        .count()
}

#[test]
fn encounter_coverage_kargath_aggro_speech_and_native_encounter() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = kargath_copy(difficulty);
        stand_near_kargath(&mut h, boss);
        h.engage(boss);
        h.advance(0.2);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KARGATH_BIT),
            Some(EncounterState::InProgress)
        );
        assert_eq!(kargath_yells(&h, boss, 0), 1, "source SAY_AGGRO once");
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1938 && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).chat.is_empty() && h.inbox(1).starts.is_empty());
    }
}

fn observe_requests(h: &mut Harness, actor: Entity) {
    let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .insert_resource(crate::scripted_ai::tests::cast_admission::CastAdmissions {
            actor,
            origin,
            entries: Vec::new(),
        });
    h.app.world_mut().entity_mut(actor).insert(
        crate::scripted_ai::tests::cast_admission::GroundCombatSeed(0),
    );
}

fn request_times(h: &Harness, spell: u32) -> Vec<f32> {
    h.app
        .world()
        .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
        .entries
        .iter()
        .filter(|entry| entry.spell == spell)
        .map(|entry| entry.time)
        .collect()
}

fn owned_actors(h: &mut Harness, boss: Entity, entry: u32) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &crate::zones::TempSummon)>()
        .iter(h.app.world())
        .filter_map(|(actor, npc, owner)| {
            (owner.summoner == boss && npc.template_id == entry).then_some(actor)
        })
        .collect()
}

#[test]
fn encounter_coverage_kargath_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = kargath_copy(difficulty);
        assert_eq!(
            h.creatures_of(20709).len(),
            28,
            "actual TDB Blade Dance markers"
        );
        assert!(crate::scripted_ai::model_reach::combat_reach(h.app.world(), boss).is_some());
        let templates = &h.app.world().resource::<WorldData>().templates;
        assert_eq!(
            templates[&17611].script_name, "npc_warchiefs_portal",
            "actual plural source binding"
        );
        assert_eq!(templates[&17695].name, "Shattered Hand Assassin");
        let spells = h.app.world().resource::<SpellStore>();
        for spell in [30738, 30739, 30751, 25821] {
            assert!(spells.get(spell).is_some(), "native70448 spell{spell}");
        }
        for spell in [30737, 30785, 30786] {
            assert!(
                spells.get(spell).is_none(),
                "named absent70448 summon{spell}"
            );
        }
        let texts = &h.app.world().resource::<CreatureTexts>().groups[&KARGATH];
        assert_eq!(
            (0..=5).map(|group| texts[&group].len()).collect::<Vec<_>>(),
            [3, 2, 1, 1, 1, 1]
        );
    }
}

#[test]
fn encounter_coverage_kargath_native_selector_really_casts_for_one_second() {
    let (mut h, boss, _) = kargath_copy(1);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30738, Some(boss), false)
        .expect("ordinary native selector");
    h.advance(0.5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(boss)
            .map(|cast| cast.spell_id),
        Some(30738)
    );
    assert!(
        spell_gos(&h, boss, 30738).is_empty(),
        "no fake synchronous hit at500ms"
    );
    h.advance(0.6);
    let gos = spell_gos(&h, boss, 30738);
    assert_eq!(gos.len(), 1, "actual native completion");
    assert!(
        !gos[0].hit_targets.is_empty(),
        "actual selector hits native markers"
    );
    assert!(
        gos[0].hit_targets.iter().all(|bits| {
            Entity::try_from_bits(*bits)
                .and_then(|actor| h.app.world().get::<shared::components::Npc>(actor))
                .is_some_and(|npc| npc.template_id == 20709)
        }),
        "TDB30738 condition selects only actual20709 markers"
    );
}

#[test]
fn encounter_coverage_kargath_native_blade_dance_damage_qualifies() {
    let (mut h, boss, _) = kargath_copy(1);
    stand_near_kargath(&mut h, boss);
    let player = h.player();
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30739, Some(boss), false)
        .expect("ordinary native Blade Dance");
    h.advance(0.2);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(30739)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual native area weapon damage"
    );
}

#[test]
fn encounter_coverage_kargath_authored_groups_arrive_then_assassins_wander_unengaged() {
    let (mut h, boss, _) = kargath_copy(1);
    stand_near_kargath(&mut h, boss);
    h.engage(boss);
    h.advance(0.9);
    assert!(owned_actors(&mut h, boss, 17611).is_empty());
    assert!(owned_actors(&mut h, boss, 17695).is_empty());
    h.advance(0.2);
    assert_eq!(
        owned_actors(&mut h, boss, 17611).len(),
        1,
        "source group0 at1s"
    );
    h.advance(1.8);
    assert!(owned_actors(&mut h, boss, 17695).is_empty(), "not before3s");
    h.advance(0.2);
    let assassins = owned_actors(&mut h, boss, 17695);
    assert_eq!(
        assassins.len(),
        4,
        "actual group1 members at3s, not the9 static assassins"
    );
    let origins = assassins
        .iter()
        .map(|&actor| {
            let origin = h
                .app
                .world()
                .get::<crate::creature_ai::SpawnPoint>(actor)
                .unwrap();
            (actor, origin.x, origin.z)
        })
        .collect::<Vec<_>>();
    h.advance(5.0);
    let mut moved = false;
    for &(actor, x, z) in &origins {
        let position = h.app.world().get::<Position>(actor).unwrap();
        let distance = (position.x - x).hypot(position.z - z);
        assert!(distance <= 5.05, "source MoveRandom5 bounds: {distance}");
        moved |= distance > 0.01;
        assert!(
            !h.app
                .world()
                .get::<shared::components::CombatStatus>(actor)
                .is_some_and(|combat| combat.0),
            "override does not zone-engage assassins"
        );
    }
    assert!(
        moved,
        "actual default random movement, not only a component"
    );
}

#[test]
fn encounter_coverage_kargath_source_dance_clock_preserves_native_selector_gap() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = kargath_copy(difficulty);
        stand_near_kargath(&mut h, boss);
        observe_requests(&mut h, boss);
        h.engage(boss);
        h.advance(95.0);
        let times = request_times(&h, 30738);
        eprintln!(
            "KARGATH difficulty{difficulty} selector requests={times:?} charge={:?}",
            request_times(&h, 25821)
        );
        assert_eq!(
            times.len(),
            16,
            "two source eight-step request bursts; busy native casts are retained, never forced"
        );
        assert!(
            times[0] >= 30.0 && times[0] <= 35.15,
            "source first30–35: {times:?}"
        );
        for burst in times.chunks_exact(8) {
            assert!(
                burst
                    .windows(2)
                    .all(|pair| (pair[1] - pair[0] - 0.5).abs() <= 0.06),
                "source500ms request clock, not fake successful admissions: {burst:?}"
            );
        }
        let admissions = &h
            .app
            .world()
            .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
            .entries;
        assert!(
            admissions.iter().any(|entry| entry.spell == 30738
                && matches!(
                    entry.result,
                    Err(crate::spell_cast::CastFailure::InProgress)
                )),
            "native1000ms cast blocks source500ms retries"
        );
        let pause = times[8] - times[0];
        assert!(
            (49.0..=54.2).contains(&pause),
            "eight500ms steps then45–50s: {times:?}"
        );
        assert!(
            request_times(&h, 30751).is_empty() && request_times(&h, 30739).is_empty(),
            "named synchronous-read gap, never replace30738 with instant selection"
        );
        let charges = request_times(&h, 25821);
        if difficulty == 2 {
            assert_eq!(charges.len(), 2);
            assert_eq!(
                spell_gos(&h, boss, 25821).len(),
                2,
                "actual ordinary Heroic charge launches, not requests alone"
            );
            assert!(
                (charges[0] - times[0] - 4.5).abs() <= 0.1,
                "Heroic500ms after source ninth timer"
            );
        } else {
            assert!(charges.is_empty());
        }
    }
}

#[test]
fn encounter_coverage_kargath_portal_keeps_source_order_without_fake_adds() {
    let (mut h, boss, _) = kargath_copy(1);
    stand_near_kargath(&mut h, boss);
    h.engage(boss);
    let mut portal = None;
    for _ in 0..24 {
        h.advance(0.05);
        if let Some(actor) = owned_actors(&mut h, boss, 17611).first().copied() {
            portal = Some(actor);
            break;
        }
    }
    let portal = portal.expect("source portal group at1s");
    observe_requests(&mut h, portal);
    h.advance(80.2);
    let entries = &h
        .app
        .world()
        .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
        .entries;
    assert_eq!(
        entries.iter().map(|entry| entry.spell).collect::<Vec<_>>(),
        [30737, 30785, 30786, 30737]
    );
    assert!(
        entries.iter().all(|entry| matches!(
            entry.result,
            Err(crate::spell_cast::CastFailure::UnknownSpell)
        )),
        "all are actual absent70448 failures, no replacement effects"
    );
    assert!(
        (entries[0].time - 20.0).abs() <= 0.1,
        "first20s after actual birth"
    );
    assert!(
        entries
            .windows(2)
            .all(|pair| (pair[1].time - pair[0].time - 20.0).abs() <= 0.06),
        "source repeat20s order"
    );
}

#[test]
fn encounter_coverage_kargath_leash_subtracts_native_combat_reach() {
    let (mut h, boss, map) = kargath_copy(1);
    stand_near_kargath(&mut h, boss);
    h.engage(boss);
    h.advance(0.2);
    let spawn = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(boss)
        .unwrap();
    let home = Position {
        x: spawn.x,
        y: spawn.y,
        z: spawn.z,
    };
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), boss).unwrap();
    let player = h.player();
    // HARNESS SETUP: both stand at the source40yd + native reach boundary.
    let edge = Position {
        x: home.x + 40.0 + reach,
        ..home
    };
    h.app.world_mut().entity_mut(boss).insert(edge);
    h.app.world_mut().entity_mut(player).insert(edge);
    h.advance(1.0);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, KARGATH_BIT),
        Some(EncounterState::InProgress)
    );
    assert_eq!(
        kargath_yells(&h, boss, 5),
        0,
        "strict greater-than boundary"
    );
    let outside = Position {
        x: edge.x + 0.5,
        ..edge
    };
    h.app.world_mut().entity_mut(boss).insert(outside);
    h.app.world_mut().entity_mut(player).insert(outside);
    h.advance(1.0);
    assert_eq!(kargath_yells(&h, boss, 5), 1, "source leash speech");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, KARGATH_BIT),
        Some(EncounterState::NotStarted)
    );
}

#[test]
fn encounter_coverage_kargath_death_speech_credit_and_owned_summon_cleanup() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = kargath_copy(difficulty);
        stand_near_kargath(&mut h, boss);
        h.engage(boss);
        h.advance(3.2);
        let mut summons = owned_actors(&mut h, boss, 17611);
        summons.extend(owned_actors(&mut h, boss, 17695));
        assert_eq!(summons.len(), 5);
        h.hit_pct(boss, 200.0);
        h.advance(0.2);
        assert_eq!(kargath_yells(&h, boss, 2), 1);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KARGATH_BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << KARGATH_BIT
        );
        assert!(
            summons
                .iter()
                .all(|&actor| h.app.world().get_entity(actor).is_err()),
            "source BossAI cleans only its five summons"
        );
        assert!(h.inbox(1).ends.is_empty() && h.inbox(1).chat.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(25.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "silent corpse");
    }
}

#[test]
fn encounter_coverage_kargath_native_destination_charge_moves_to_actual_marker() {
    let (mut h, boss, _) = kargath_copy(1);
    let start = *h.app.world().get::<Position>(boss).unwrap();
    let marker = h
        .creatures_of(20709)
        .into_iter()
        .filter(|&actor| {
            let position = h.app.world().get::<Position>(actor).unwrap();
            let distance = Vec3::new(
                position.x - start.x,
                position.y - start.y,
                position.z - start.z,
            )
            .length();
            (10.0..35.0).contains(&distance)
        })
        .min_by(|&a, &b| {
            let distance = |actor| {
                let p = h.app.world().get::<Position>(actor).unwrap();
                Vec3::new(p.x - start.x, p.y - start.y, p.z - start.z).length_squared()
            };
            distance(a).total_cmp(&distance(b))
        })
        .expect("authored marker10–35yd from Kargath");
    let destination = *h.app.world().get::<Position>(marker).unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30751, Some(marker), false)
        .expect("ordinary native30751");
    h.advance(0.05);
    let first = *h.app.world().get::<Position>(boss).unwrap();
    let first_step = Vec3::new(first.x - start.x, first.y - start.y, first.z - start.z).length();
    assert!(
        first_step > 0.0 && first_step <= 42.0 * 0.05 + 0.02,
        "native default42yd/s at50ms, neither ignored nor teleporting: {first_step}"
    );
    h.advance(1.0);
    let end = *h.app.world().get::<Position>(boss).unwrap();
    assert!(
        Vec3::new(
            end.x - destination.x,
            end.y - destination.y,
            end.z - destination.z
        )
        .length()
            <= 0.05,
        "destination charge ends at source marker, not a reach-offset substitute: {end:?} vs {destination:?}"
    );
}

#[test]
fn encounter_coverage_kargath_slay_speaks_only_for_actual_player_kills() {
    for player_victim in [true, false] {
        let (mut h, boss, _) = kargath_copy(1);
        super::omor_slay::install_player_death_transition(&mut h);
        stand_near_kargath(&mut h, boss);
        h.engage(boss);
        h.advance(0.2);
        let victim = if player_victim {
            h.player()
        } else {
            h.creatures_of(20709)[0]
        };
        // HARNESS SETUP: source explicit Unit::Kill, never a fabricated KilledUnit hook.
        crate::combat::kill_unit(h.app.world_mut(), boss, victim);
        h.advance(0.2);
        assert_eq!(
            kargath_yells(&h, boss, 1),
            usize::from(player_victim),
            "source TYPEID_PLAYER filter"
        );
        h.advance(0.5);
        assert_eq!(
            kargath_yells(&h, boss, 1),
            usize::from(player_victim),
            "no corpse replay"
        );
        assert!(h.inbox(1).chat.is_empty());
    }
}
