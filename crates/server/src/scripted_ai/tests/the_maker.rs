//! TC a352b1fa Blood Furnace The Maker17381 (boss_the_maker.cpp).
use super::*;

const MAKER: u32 = 17381;
const BLOOD_FURNACE: u32 = 542;
const NORMAL_BEAKER: u32 = 30925;
const HEROIC_BEAKER: u32 = 40059;
const DOMINATION: u32 = 30923;

/// Like Ramparts, the boss slot is the fixture-only ordinal0 binding; production
/// world data publishes no Blood Furnace creature binding yet.
fn maker_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(MAKER, BLOOD_FURNACE, 0, difficulty, None);
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&BLOOD_FURNACE].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(BLOOD_FURNACE, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    (h, boss, map)
}

#[test]
#[ignore = "RED: Maker domination30923 is not scheduled"]
fn charm_control_native_maker_30923_charms_a_player_only_with_multiple_player_threats() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = pull_maker(difficulty);
        // The production server installs PetControlPlugin; this isolated boss harness
        // has no pets, so install its same AI consumer for the charmed player.
        h.app.add_systems(
            FixedUpdate,
            crate::pet_control::pet_ai_system.after(crate::combat::auto_attack_system),
        );
        let at = *h.app.world().get::<Position>(h.player()).unwrap();
        let (x, y, z) = crate::terrain_height::bevy_to_wow(at.x + 1.0, at.y, at.z);
        let (second, connection) = spawn_player(&mut h.app, map, [x, y, z, 0.0], 3);
        h.players.push((second, connection, Inbox::default()));
        crate::creature_aggro::add_threat(h.app.world_mut(), boss, second, 1.0);
        let mut charmed = None;
        for tick in 1..=800 {
            h.advance(0.05);
            for player in [h.player(), second] {
                if h.has_aura(player, DOMINATION) {
                    charmed = Some((player, tick as f32 * 0.05));
                    break;
                }
            }
            if charmed.is_some() {
                break;
            }
        }
        let (player, at) =
            charmed.expect("source Maker must cast native30923 at a player in20-30s");
        assert!(
            (20.0..=31.0).contains(&at),
            "native first domination window {at}"
        );
        let charm = h
            .app
            .world()
            .get::<shared::components::UnitCharm>(player)
            .unwrap();
        assert_eq!(charm.charmer, boss.to_bits());
        assert!(
            !charm.possessed,
            "Maker uses aura6 AI control, not a player mover"
        );
        assert_eq!(
            h.app.world().get::<UnitFactionTemplate>(player),
            h.app.world().get::<UnitFactionTemplate>(boss)
        );
        assert!(
            h.app
                .world()
                .get::<shared::components::MovementControl>(player)
                .unwrap()
                .controlled
        );
        let victim = if player == h.player() {
            second
        } else {
            h.player()
        };
        h.advance(3.0);
        assert!(
            h.players
                .iter()
                .any(|(_, _, inbox)| inbox.log.iter().any(|event| {
                    event.kind == shared::protocol::CombatLogKind::Damage
                        && event.source == Some(player.to_bits())
                        && event.target == Some(victim.to_bits())
                        && event.amount > 0
                })),
            "Maker-controlled player must actually strike the remaining party member"
        );
        crate::auras::remove_auras(
            h.app.world_mut(),
            player,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == DOMINATION,
        );
        assert_eq!(
            h.app.world().get::<UnitFactionTemplate>(player).unwrap().0,
            1
        );
        assert!(
            !h.app
                .world()
                .get::<shared::components::MovementControl>(player)
                .unwrap()
                .controlled
        );
    }
    let (mut solo, boss, _) = pull_maker(1);
    solo.advance(35.0);
    assert!(
        spell_gos(&solo, boss, DOMINATION).is_empty(),
        "TC refuses domination with one player threat"
    );
}

/// Advance `secs`, reporting whether `target` was ever in a knockback flight.
fn observe_knockback(h: &mut Harness, target: Entity, secs: f32) -> bool {
    let mut knocked = false;
    for _ in 0..(secs / 0.05).round() as usize {
        h.advance(0.05);
        knocked |= h
            .app
            .world()
            .get::<crate::knockback::KnockbackFlight>(target)
            .is_some();
    }
    knocked
}

#[test]
fn encounter_coverage_maker_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = maker_copy(difficulty);
        let world = h.app.world();
        assert_eq!(
            world
                .resource::<crate::maps::MapStore>()
                .instance_scripts
                .get(&BLOOD_FURNACE)
                .map(String::as_str),
            Some("instance_blood_furnace")
        );
        assert_eq!(
            world
                .get::<shared::components::Npc>(boss)
                .unwrap()
                .template_id,
            MAKER,
            "TDB spawnDifficulties1,2 spawn in difficulty{difficulty}"
        );
        let texts = &world.resource::<CreatureTexts>().groups[&MAKER];
        let counts: Vec<_> = (0..=2).map(|group| texts[&group].len()).collect();
        assert_eq!(counts, [3, 2, 1], "native aggro/slay/death groups");
        assert!(
            texts
                .values()
                .flatten()
                .all(|text| text.chat_type == 14 && text.text_range == 0),
            "native MonsterYell/TextRange0"
        );
        let spells = world.resource::<SpellStore>();
        assert!(
            spells.get(NORMAL_BEAKER).is_none(),
            "retail70448 has no Normal beaker30925"
        );
        assert!(spells.get(HEROIC_BEAKER).is_some());
        assert!(spells.get(DOMINATION).is_some());
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::InProgress)
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1922 && start.difficulty_id == difficulty),
            "ordinal0 -> DungeonEncounter1922 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_maker_native_heroic_beaker_and_normal_absence_qualify() {
    let (mut h, boss, _) = maker_copy(2);
    let player = h.player();
    let before = *h.app.world().get::<Position>(player).unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, HEROIC_BEAKER, Some(player), false)
        .expect("actual ordinary Heroic40059 admission");
    let knocked = observe_knockback(&mut h, player, 4.0);
    eprintln!(
        "MAKER beaker gos={:?} log={:?}",
        spell_gos(&h, boss, HEROIC_BEAKER),
        h.inbox(0)
            .log
            .iter()
            .filter(|event| event.spell_id == Some(HEROIC_BEAKER))
            .collect::<Vec<_>>()
    );
    assert_eq!(spell_gos(&h, boss, HEROIC_BEAKER).len(), 1);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(HEROIC_BEAKER)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual area enemy damage at victim"
    );
    let after = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        knocked && after != before,
        "actual Knockback98 launches the victim"
    );

    let (mut h, boss, _) = maker_copy(1);
    let player = h.player();
    assert!(matches!(
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            boss,
            NORMAL_BEAKER,
            Some(player),
            false
        ),
        Err(crate::spell_cast::CastFailure::UnknownSpell)
    ));
    h.advance(1.0);
    assert!(spell_gos(&h, boss, NORMAL_BEAKER).is_empty());
}

/// The Maker in `difficulty`, engaged by the first player, with chat visibility.
fn pull_maker(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = maker_copy(difficulty);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    (h, boss, map)
}

fn count_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&MAKER][&group];
    let name = &h
        .app
        .world()
        .get::<shared::components::Npc>(boss)
        .unwrap()
        .name;
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            let authored = texts
                .iter()
                .any(|text| line.content == text.male_text || line.content == text.female_text);
            authored
                && line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && &line.sender == name
        })
        .count()
}

/// Advance `secs`, returning simulated times of `caster`'s new `spell` SpellGos.
fn cast_times(h: &mut Harness, caster: Entity, spell: u32, secs: f32) -> Vec<f32> {
    let mut cursor = h.inbox(0).spell_gos.len();
    let mut times = Vec::new();
    for tick in 1..=(secs / 0.05).round() as usize {
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        let new = gos[cursor..]
            .iter()
            .filter(|go| go.caster == caster.to_bits() && go.spell_id == spell)
            .count();
        times.extend(std::iter::repeat_n(tick as f32 * 0.05, new));
        cursor = gos.len();
    }
    times
}

fn assert_beaker_window(times: &[f32], first: (f32, f32)) {
    assert!(times.len() >= 3, "source first+repeats in60s: {times:?}");
    assert!(
        times[0] >= first.0 && times[0] <= first.1 + 0.2,
        "first beaker5–15s: {times:?}"
    );
    assert!(
        times
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= 9.9 && pair[1] - pair[0] <= 20.2),
        "Repeat10–20s: {times:?}"
    );
}

#[test]
fn encounter_fixture_cast_progress_starts_after_admission_tick() {
    for _ in 0..4 {
        let (mut h, boss, _) = maker_copy(2);
        for _ in 0..400 {
            h.advance(0.05);
            if let Some(cast) = h.app.world().get::<CastState>(boss) {
                assert_eq!(cast.spell_id, HEROIC_BEAKER);
                assert_eq!(
                    cast.elapsed, 0.0,
                    "new cast cannot consume the admission tick"
                );
                h.advance(0.45);
                assert!(h.app.world().get::<CastState>(boss).is_some());
                h.advance(0.05);
                assert!(h.app.world().get::<CastState>(boss).is_none());
                break;
            }
        }
        assert!(!spell_gos(&h, boss, HEROIC_BEAKER).is_empty());
    }
}

#[test]
fn encounter_fixture_seed_precedes_automatic_aggro() {
    let (mut first, first_boss, _) = maker_copy(2);
    let first_times = cast_times(&mut first, first_boss, HEROIC_BEAKER, 60.0);
    std::thread::sleep(Duration::from_millis(300));
    let (mut second, second_boss, _) = maker_copy(2);
    let second_times = cast_times(&mut second, second_boss, HEROIC_BEAKER, 60.0);
    assert!(first_times.len() >= 3, "real heroic beaker casts");
    assert_eq!(
        first_times, second_times,
        "fixture RNG must be seeded before the actor can acquire its victim"
    );
}

#[test]
fn encounter_coverage_maker_heroic_aggro_speech_and_beaker_schedule() {
    let (mut h, boss, _) = pull_maker(2);
    let player = h.player();
    let origin = h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64();
    h.app.insert_resource(cast_admission::CastAdmissions {
        actor: boss,
        origin,
        entries: Vec::new(),
    });
    let times = cast_times(&mut h, boss, HEROIC_BEAKER, 60.0);
    let starts: Vec<_> = h
        .app
        .world()
        .resource::<cast_admission::CastAdmissions>()
        .entries
        .iter()
        .filter(|entry| entry.spell == HEROIC_BEAKER && entry.result.is_ok())
        .map(|entry| entry.time)
        .collect();
    eprintln!("MAKER heroic beaker starts={starts:?} impacts={times:?}");
    assert_eq!(count_yells(&h, boss, 0), 1, "JustEngagedWith Talk0 once");
    // TC schedules DoCastVictim at5–15s; SpellGo follows the native500ms cast.
    assert_beaker_window(&starts, (5.0, 15.0));
    assert_eq!(starts.len(), times.len(), "every admitted cast completes");
    for (start, impact) in starts.iter().zip(&times) {
        let cast_time = impact - start;
        assert_eq!(
            (cast_time * 1000.0).round() as u32,
            500,
            "native500ms cast: {cast_time}"
        );
    }
    assert!(
        spell_gos(&h, boss, HEROIC_BEAKER)
            .iter()
            .all(|go| go.target == Some(player.to_bits())),
        "DoCastVictim"
    );
    let damage = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(HEROIC_BEAKER)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(damage, times.len(), "each actual beaker impact damages");
    assert!(
        spell_gos(&h, boss, DOMINATION).is_empty(),
        "source Domination requires more than one threat player"
    );
    assert!(h.inbox(1).chat.is_empty(), "other copy cannot hear");
}

#[test]
fn encounter_coverage_maker_normal_beaker_absent_without_substitute() {
    let (mut h, boss, map) = pull_maker(1);
    h.advance(60.0);
    assert_eq!(count_yells(&h, boss, 0), 1, "JustEngagedWith Talk0 once");
    // MAKER_NORMAL_BEAKER_30925_ABSENT_70448: no SpellInfo, no substitute spell.
    for spell in [NORMAL_BEAKER, HEROIC_BEAKER] {
        assert!(spell_gos(&h, boss, spell).is_empty(), "no beaker{spell}");
    }
    assert!(h.health(boss).current > 0.0);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::InProgress)
    );
}

#[test]
fn encounter_coverage_maker_slay_speaks_for_autonomous_player_kill() {
    let (mut h, boss, _) = maker_copy(1);
    super::omor_slay::install_player_death_transition(&mut h);
    super::vazruden_slay::apply_native_low_level_stats(&mut h);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    let player = h.player();
    for _ in 0..3600 {
        h.advance(0.05);
        if h.health(player).current <= 0.0 {
            break;
        }
    }
    assert_eq!(h.health(player).current, 0.0, "native Normal melee kill");
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        }),
        "autonomous ordinary melee, no forced kill"
    );
    let death = h
        .app
        .world()
        .get::<crate::death::PlayerDeathState>(player)
        .expect("production player death transition");
    assert!(death.corpse.is_some());
    assert_eq!(count_yells(&h, boss, 1), 1, "KilledUnit(player) Talk1");
    h.advance(1.0);
    assert_eq!(count_yells(&h, boss, 1), 1, "no corpse replay");
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
fn encounter_coverage_maker_slay_ignores_npc_victim() {
    let (mut h, boss, map) = pull_maker(1);
    h.advance(0.5);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (enforcer, _) = spawn_authored_actor(&mut h.app, &conn, 17370, map, 1);
    h.advance(0.1);
    // HARNESS SETUP: production Unit::Kill attributes the real17370 death to The Maker.
    crate::combat::kill_unit(h.app.world_mut(), boss, enforcer);
    h.advance(0.5);
    assert_eq!(h.health(enforcer).current, 0.0);
    assert_eq!(count_yells(&h, boss, 1), 0, "source filters non-players");
}

#[test]
fn encounter_coverage_maker_death_speech_and_credit() {
    let (mut h, boss, map) = pull_maker(2);
    h.advance(1.0);
    h.hit_pct(boss, 200.0);
    h.advance(0.2);
    assert_eq!(count_yells(&h, boss, 2), 1, "JustDied Talk2");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 0),
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
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 1922 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty() && h.inbox(1).chat.is_empty());
    let before = spell_gos(&h, boss, HEROIC_BEAKER).len();
    h.advance(30.0);
    assert_eq!(
        spell_gos(&h, boss, HEROIC_BEAKER).len(),
        before,
        "silent corpse"
    );
    assert_eq!(count_yells(&h, boss, 2), 1);
}

#[test]
fn encounter_coverage_maker_evade_rearms_aggro_and_beaker() {
    let (mut h, boss, map) = pull_maker(2);
    let first = cast_times(&mut h, boss, HEROIC_BEAKER, 15.2);
    assert_eq!(first.len(), 1, "first pull beaker: {first:?}");
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 500.0,
        ..origin
    });
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    for _ in 0..400 {
        h.advance(0.05);
        if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
            != Some(&crate::creature_ai::CreatureState::Returning)
        {
            break;
        }
    }
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 0),
        Some(EncounterState::NotStarted)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 1922 && !end.success)
    );
    let home = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: home.x + 3.0,
        ..home
    });
    h.engage(boss);
    let again = cast_times(&mut h, boss, HEROIC_BEAKER, 15.2);
    assert_eq!(count_yells(&h, boss, 0), 2, "re-pull Talk0");
    assert_eq!(again.len(), 1, "re-armed first beaker5–15s: {again:?}");
    assert!(again[0] >= 5.0, "evade cleared the old schedule: {again:?}");
}
