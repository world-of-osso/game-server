//! TC a352b1fa Blood Furnace Keli'dan the Breaker17377 (boss_kelidan_the_breaker.cpp).
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

const KELIDAN: u32 = 17377;
const CHANNELER: u32 = 17653;
const BLOOD_FURNACE: u32 = 542;
/// TC DATA_KELIDAN: installed DB2 encounter1923, completion bit1.
const KELIDAN_BIT: u32 = 1;
const CHANNELER_GROUPS: [u32; 5] = [360, 361, 362, 363, 364];
const EVOCATION: u32 = 30935;
const SHADOW_BOLT_VOLLEY: u32 = 17228;
const SHADOW_BOLT_VOLLEY_H: u32 = 40070;
const CORRUPTION: u32 = 30938;
const BURNING_NOVA: u32 = 30940;
const STAR_BEAM: u32 = 30888;
const SHADOW_BOLT: u32 = 12739;
const SHADOW_BOLT_H: u32 = 15472;
const MARK_OF_SHADOW: u32 = 30937;
const VORTEX: u32 = 37370;
const FIRE_NOVA: u32 = 33775;
const FIRE_NOVA_H: u32 = 37371;

/// A Blood Furnace copy filled by production copy fill from the real map542 rows,
/// with authentic ground and eye tables.
fn kelidan_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        KELIDAN,
        BLOOD_FURNACE,
        KELIDAN_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
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
    (h, map)
}

/// The first player sees `actors` (chat, SpellGo and combat log delivery).
fn watch(h: &mut Harness, actors: &[Entity]) {
    let connection = h.players[0].1;
    for &actor in actors {
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(connection);
        h.app.world_mut().entity_mut(actor).insert(visibility);
    }
}

/// Every logged `spell` CastSuccess by `caster`, including events no player receives.
fn observed_casts(h: &Harness, caster: Entity, spell: u32) -> usize {
    h.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastSuccess
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(spell)
        })
        .count()
}

fn spawn_string_id(h: &Harness, actor: Entity) -> Option<String> {
    h.app
        .world()
        .get::<crate::zones::CreatureStringIds>(actor)
        .and_then(|ids| ids.spawn.clone())
}

/// The copy's channeler carrying spawn StringId `KelidansShadowmoonChannelerN`.
fn channeler(h: &mut Harness, n: u32) -> Entity {
    let wanted = format!("KelidansShadowmoonChanneler{n}");
    let channelers = h.creatures_of(CHANNELER);
    channelers
        .into_iter()
        .find(|&actor| spawn_string_id(h, actor).as_deref() == Some(wanted.as_str()))
        .unwrap_or_else(|| panic!("copy has {wanted}"))
}

#[test]
fn encounter_coverage_kelidan_native_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let groups =
        crate::world_data::load_script_spawn_groups(&conn, &CHANNELER_GROUPS, true).unwrap();
    let strings = crate::world_data::load_spawn_string_ids(&conn).unwrap();
    for (n, group) in CHANNELER_GROUPS.iter().enumerate() {
        let guids = &groups[group];
        assert_eq!(guids.len(), 1, "manual group{group} holds one channeler");
        assert_eq!(
            strings.get(&guids[0]).map(String::as_str),
            Some(format!("KelidansShadowmoonChanneler{}", n + 1).as_str()),
            "group{group} member StringId"
        );
    }
    for difficulty in [1, 2] {
        let (mut h, map) = kelidan_copy(difficulty);
        assert_eq!(
            h.app
                .world()
                .resource::<MapStore>()
                .instance_scripts
                .get(&BLOOD_FURNACE)
                .map(String::as_str),
            Some("instance_blood_furnace")
        );
        let bosses = h.creatures_of(KELIDAN);
        assert_eq!(
            bosses.len(),
            1,
            "TDB spawnDifficulties1,2 in difficulty{difficulty}"
        );
        let boss = bosses[0];
        let channelers = h.creatures_of(CHANNELER);
        eprintln!(
            "KELIDAN difficulty{difficulty} copy-fill channelers={} ids={:?}",
            channelers.len(),
            channelers
                .iter()
                .map(|&actor| spawn_string_id(&h, actor))
                .collect::<Vec<_>>()
        );
        let texts = &h.app.world().resource::<CreatureTexts>().groups;
        let counts: Vec<_> = (0..=3).map(|group| texts[&KELIDAN][&group].len()).collect();
        assert_eq!(counts, [1, 1, 2, 1], "native wake/nova/slay/death groups");
        assert!(
            texts[&KELIDAN]
                .values()
                .flatten()
                .all(|text| text.chat_type == 14),
            "Keli'dan MonsterYell"
        );
        assert_eq!(texts[&CHANNELER][&0].len(), 3, "channeler aggro yells");
        assert_eq!(texts[&CHANNELER][&1].len(), 7, "channeler aggro says");
        let spells = h.app.world().resource::<SpellStore>();
        assert!(
            spells.get(SHADOW_BOLT_VOLLEY_H).is_none(),
            "retail70448 has no Heroic volley40070"
        );
        for spell in [
            EVOCATION,
            SHADOW_BOLT_VOLLEY,
            CORRUPTION,
            BURNING_NOVA,
            STAR_BEAM,
            SHADOW_BOLT,
            SHADOW_BOLT_H,
            MARK_OF_SHADOW,
            VORTEX,
            FIRE_NOVA,
            FIRE_NOVA_H,
        ] {
            assert!(spells.get(spell).is_some(), "spell{spell}");
        }
        // HARNESS SETUP: direct threat, not the source channeler release.
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
            Some(EncounterState::InProgress)
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1923 && start.difficulty_id == difficulty),
            "bit1 -> DungeonEncounter1923 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_kelidan_native_spell_admissions_qualify() {
    for difficulty in [1, 2] {
        let (mut h, _) = kelidan_copy(difficulty);
        h.app.init_resource::<ObservedCombatEvents>();
        let boss = h.creatures_of(KELIDAN)[0];
        let first = channeler(&mut h, 1);
        let partner = channeler(&mut h, 4);
        watch(&mut h, &[boss, first, partner]);
        let world = h.app.world_mut();
        crate::spell_cast::creature_cast(world, boss, EVOCATION, None, false)
            .expect("ordinary self Evocation");
        crate::spell_cast::creature_cast(world, first, STAR_BEAM, Some(partner), false)
            .expect("ordinary channeler1 -> channeler4 Star Beam");
        h.advance(0.5);
        assert!(h.has_aura(boss, EVOCATION));
        assert!(h.has_aura(partner, STAR_BEAM));

        let nova = if difficulty == 2 {
            FIRE_NOVA_H
        } else {
            FIRE_NOVA
        };
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, nova, None, false)
            .expect("ordinary Fire Nova");
        if difficulty == 2 {
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, VORTEX, None, true)
                .expect("triggered Heroic Vortex");
        }
        h.advance(5.5);
        eprintln!(
            "KELIDAN difficulty{difficulty} nova casts={} vortex casts={}",
            observed_casts(&h, boss, nova),
            observed_casts(&h, boss, VORTEX)
        );
        assert_eq!(observed_casts(&h, boss, nova), 1, "Fire Nova completes");
        if difficulty == 2 {
            assert_eq!(observed_casts(&h, boss, VORTEX), 1, "Vortex resolves");
        }

        // After the direct admissions: the ported aura script casts on expiry.
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, BURNING_NOVA, None, false)
            .expect("ordinary self Burning Nova");
        h.advance(0.5);
        assert!(h.has_aura(boss, BURNING_NOVA), "5s self stun aura");
        h.advance(5.0);
        assert!(!h.has_aura(boss, BURNING_NOVA), "stun expires");
    }
}

/// TC GetStringIdOfChanneler: channeler N beams the channeler carrying this id.
const STAR_BEAM_PARTNER: [(u32, u32); 5] = [(1, 4), (2, 3), (3, 5), (4, 2), (5, 1)];

/// A cast event at simulated seconds after observation started.
struct Timed {
    at: f32,
    kind: CombatLogKind,
    source: Option<u64>,
    target: Option<u64>,
    spell: Option<u32>,
}

/// Advance `secs` in50ms ticks, timestamping every newly logged combat event.
fn observe(h: &mut Harness, secs: f32) -> Vec<Timed> {
    observe_with(h, secs, |_, _| {})
}

/// `observe`, calling `on_tick` with the simulated time after each tick.
fn observe_with(h: &mut Harness, secs: f32, mut on_tick: impl FnMut(&Harness, f32)) -> Vec<Timed> {
    h.app.init_resource::<ObservedCombatEvents>();
    let mut cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    let mut timed = Vec::new();
    for tick in 1..=(secs / 0.05).round() as usize {
        h.advance(0.05);
        let at = tick as f32 * 0.05;
        on_tick(h, at);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        timed.extend(events[cursor..].iter().map(|event| Timed {
            at,
            kind: event.kind.clone(),
            source: event.source,
            target: event.target,
            spell: event.spell_id,
        }));
        cursor = events.len();
    }
    timed
}

fn times(events: &[Timed], kind: CombatLogKind, caster: Entity, spell: u32) -> Vec<f32> {
    events
        .iter()
        .filter(|event| {
            event.kind == kind
                && event.source == Some(caster.to_bits())
                && event.spell == Some(spell)
        })
        .map(|event| event.at)
        .collect()
}

/// Lines `speaker` said from its template's text `group`, heard by the first player.
fn spoken(h: &Harness, speaker: Entity, entry: u32, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&entry][&group];
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            texts
                .iter()
                .any(|text| line.content == text.male_text || line.content == text.female_text)
                && matches!(line.channel,
                    shared::protocol::ChatType::MonsterYell(bits)
                    | shared::protocol::ChatType::MonsterSay(bits) if bits == speaker.to_bits())
        })
        .count()
}

/// The first player waits100yd east of Keli'dan: inside yell range, outside aggro.
fn listen(h: &mut Harness, boss: Entity) {
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 100.0,
        ..origin
    });
    let mut actors = h.creatures_of(CHANNELER);
    actors.push(boss);
    watch(h, &actors);
}

fn living(h: &mut Harness, entry: u32) -> Vec<Entity> {
    let actors = h.creatures_of(entry);
    actors
        .into_iter()
        .filter(|&actor| h.health(actor).current > 0.0)
        .collect()
}

fn dormant(h: &Harness, boss: Entity) -> bool {
    let flags = h
        .app
        .world()
        .get::<shared::components::UnitFlags>(boss)
        .unwrap()
        .0;
    flags & shared::components::UnitFlags::IMMUNE_TO_NPC != 0
        && flags & shared::components::UnitFlags::NOT_SELECTABLE != 0
        && h.app.world().get::<ReactState>(boss) == Some(&ReactState::Passive)
}

/// HARNESS SETUP: production Unit::Kill of each channeler by the first player.
fn kill_channelers(h: &mut Harness, ids: impl IntoIterator<Item = u32>) {
    let player = h.player();
    for n in ids {
        let actor = channeler(h, n);
        crate::combat::kill_unit(h.app.world_mut(), player, actor);
        h.advance(0.1);
    }
}

/// Keli'dan released by the fifth channeler death, with the first player listening.
fn released(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, map) = kelidan_copy(difficulty);
    let boss = h.creatures_of(KELIDAN)[0];
    listen(&mut h, boss);
    kill_channelers(&mut h, 1..=5);
    (h, boss, map)
}

#[test]
fn encounter_coverage_kelidan_appears_dormant_with_one_set_of_channelers() {
    for difficulty in [1, 2] {
        let (mut h, map) = kelidan_copy(difficulty);
        let boss = h.creatures_of(KELIDAN)[0];
        h.advance(0.5);
        assert!(h.has_aura(boss, EVOCATION), "JustAppeared self Evocation");
        assert!(dormant(&h, boss), "ImmuneToNPC, Uninteractible, Passive");
        let mut ids: Vec<_> = living(&mut h, CHANNELER)
            .into_iter()
            .map(|actor| spawn_string_id(&h, actor).unwrap())
            .collect();
        ids.sort();
        let expected: Vec<_> = (1..=5)
            .map(|n| format!("KelidansShadowmoonChanneler{n}"))
            .collect();
        assert_eq!(
            ids, expected,
            "SpawnGroupSpawn360-364 once, no duplicate fill"
        );
        let player = h.player();
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + 2.0,
            ..origin
        });
        h.advance(3.0);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
            Some(EncounterState::NotStarted),
            "passive Keli'dan ignores an adjacent player"
        );
    }
}

#[test]
fn encounter_coverage_kelidan_channelers_star_beam_their_source_partner_after_10s() {
    let (mut h, _) = kelidan_copy(1);
    let events = observe(&mut h, 11.0);
    for (from, to) in STAR_BEAM_PARTNER {
        let caster = channeler(&mut h, from);
        let partner = channeler(&mut h, to);
        let beams: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(caster.to_bits())
                    && event.spell == Some(STAR_BEAM)
            })
            .collect();
        assert_eq!(beams.len(), 1, "channeler{from} beams once");
        assert_eq!(beams[0].target, Some(partner.to_bits()), "{from}->{to}");
        assert!(
            (9.8..=10.6).contains(&beams[0].at),
            "10s after appearance: {}",
            beams[0].at
        );
    }
}

#[test]
fn encounter_coverage_kelidan_wakes_only_after_the_fifth_channeler_dies() {
    for difficulty in [1, 2] {
        let (mut h, map) = kelidan_copy(difficulty);
        let boss = h.creatures_of(KELIDAN)[0];
        listen(&mut h, boss);
        kill_channelers(&mut h, 1..=4);
        h.advance(1.0);
        assert!(dormant(&h, boss), "four deaths keep Keli'dan dormant");
        assert_eq!(spoken(&h, boss, KELIDAN, 0), 0);
        kill_channelers(&mut h, [5]);
        h.advance(0.5);
        let flags = h
            .app
            .world()
            .get::<shared::components::UnitFlags>(boss)
            .unwrap()
            .0;
        assert_eq!(
            flags
                & (shared::components::UnitFlags::IMMUNE_TO_NPC
                    | shared::components::UnitFlags::NOT_SELECTABLE),
            0
        );
        assert_eq!(
            h.app.world().get::<ReactState>(boss),
            Some(&ReactState::Aggressive)
        );
        assert_eq!(spoken(&h, boss, KELIDAN, 0), 1, "JustEngagedWith Talk0");
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
            Some(EncounterState::InProgress),
            "DoZoneInCombat engages the copy's player"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 1923 && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_kelidan_normal_rotation_and_nova_speech() {
    let (mut h, boss, _) = released(1);
    let events = observe(&mut h, 60.0);
    let volleys = times(&events, CombatLogKind::CastStart, boss, SHADOW_BOLT_VOLLEY);
    let corruption = times(&events, CombatLogKind::CastSuccess, boss, CORRUPTION);
    let novas = times(&events, CombatLogKind::CastSuccess, boss, BURNING_NOVA);
    eprintln!("KELIDAN normal volleys={volleys:?} corruption={corruption:?} novas={novas:?}");
    assert!(
        !volleys.is_empty() && volleys[0] <= 2.2,
        "volley0-2s: {volleys:?}"
    );
    assert!(
        !corruption.is_empty() && corruption[0] >= 5.0 && corruption[0] <= 13.7,
        "Corruption5-10s plus casting delay: {corruption:?}"
    );
    assert!(
        !novas.is_empty() && novas[0] >= 10.0 && novas[0] <= 23.7,
        "Burning Nova10-20s plus casting delay: {novas:?}"
    );
    assert_eq!(
        spoken(&h, boss, KELIDAN, 1),
        novas.len(),
        "SpellHit(30940) Talk1 per nova"
    );
}

#[test]
fn encounter_coverage_kelidan_heroic_volley_absent_without_substitute() {
    let (mut h, boss, _) = released(2);
    let events = observe(&mut h, 60.0);
    // KELIDAN_HEROIC_VOLLEY_40070_ABSENT_70448: no SpellInfo, no substitute.
    for spell in [SHADOW_BOLT_VOLLEY, SHADOW_BOLT_VOLLEY_H] {
        assert!(times(&events, CombatLogKind::CastStart, boss, spell).is_empty());
        assert!(times(&events, CombatLogKind::CastSuccess, boss, spell).is_empty());
    }
    assert!(!times(&events, CombatLogKind::CastSuccess, boss, CORRUPTION).is_empty());
    assert!(!times(&events, CombatLogKind::CastSuccess, boss, BURNING_NOVA).is_empty());
}

#[test]
fn encounter_coverage_kelidan_burning_nova_aura_script_casts_vortex_and_fire_nova() {
    for difficulty in [1, 2] {
        // Dormant Keli'dan casts nothing that could hold the stimulus InProgress.
        let (mut h, _) = kelidan_copy(difficulty);
        let boss = h.creatures_of(KELIDAN)[0];
        h.advance(0.5);
        h.app.init_resource::<ObservedCombatEvents>();
        let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, BURNING_NOVA, None, false)
            .expect("ordinary self Burning Nova stimulus");
        // The instant aura applies inside the cast call, at observation time0.
        let mut events: Vec<_> = h.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .map(|event| Timed {
                at: 0.0,
                kind: event.kind.clone(),
                source: event.source,
                target: event.target,
                spell: event.spell_id,
            })
            .collect();
        events.extend(observe(&mut h, 6.0));
        let applied = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::AuraApplied && event.spell == Some(BURNING_NOVA)
            })
            .expect("5s stun aura applied")
            .at;
        let removed = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::AuraRemoved && event.spell == Some(BURNING_NOVA)
            })
            .expect("stun aura removed")
            .at;
        let vortex = times(&events, CombatLogKind::CastSuccess, boss, VORTEX);
        if difficulty == 2 {
            assert_eq!(vortex, [applied], "Heroic AfterApply triggered Vortex");
            let nova = times(&events, CombatLogKind::CastSuccess, boss, FIRE_NOVA_H);
            assert!(
                nova.len() == 1 && nova[0] >= removed && nova[0] <= removed + 0.1,
                "AfterRemove instant Heroic Fire Nova: {nova:?} removed {removed}"
            );
        } else {
            assert!(vortex.is_empty(), "Normal has no Vortex");
            let nova = times(&events, CombatLogKind::CastStart, boss, FIRE_NOVA);
            assert!(
                nova.len() == 1 && nova[0] >= removed && nova[0] <= removed + 0.1,
                "AfterRemove ordinary Fire Nova cast: {nova:?} removed {removed}"
            );
        }
    }
}

#[test]
fn encounter_coverage_kelidan_slay_and_death_speech_with_credit() {
    let (mut h, boss, _) = released(1);
    super::omor_slay::install_player_death_transition(&mut h);
    let player = h.player();
    // HARNESS SETUP: production Unit::Kill attributes the player's death to Keli'dan.
    crate::combat::kill_unit(h.app.world_mut(), boss, player);
    h.advance(0.5);
    assert_eq!(spoken(&h, boss, KELIDAN, 2), 1, "KilledUnit Talk2");

    let (mut h, boss, map) = released(2);
    h.advance(1.0);
    h.hit_pct(boss, 200.0);
    h.advance(0.2);
    assert_eq!(spoken(&h, boss, KELIDAN, 3), 1, "JustDied Talk3");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 1923 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty() && h.inbox(1).chat.is_empty());
}

#[test]
fn encounter_coverage_kelidan_reset_redormant_reevocates_and_respawns_channelers() {
    let (mut h, boss, map) = released(1);
    h.advance(2.0);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 500.0,
        ..origin
    });
    // HARNESS SETUP: production evade of the engaged boss.
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    let mut home_at = None;
    let events = observe_with(&mut h, 20.0, |h, at| {
        if home_at.is_none()
            && h.app.world().get::<crate::creature_ai::CreatureState>(boss)
                != Some(&crate::creature_ai::CreatureState::Returning)
        {
            home_at = Some(at);
        }
    });
    let home_at = home_at.expect("Keli'dan reaches home");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, KELIDAN_BIT),
        Some(EncounterState::NotStarted)
    );
    assert!(dormant(&h, boss), "JustReachedHome restores dormant flags");
    let evocations = times(&events, CombatLogKind::CastSuccess, boss, EVOCATION);
    assert!(
        evocations.len() == 1 && evocations[0] >= home_at + 4.9 && evocations[0] <= home_at + 5.3,
        "EVENT_EVOCATION5s after home {home_at}: {evocations:?}"
    );
    let mut ids: Vec<_> = living(&mut h, CHANNELER)
        .into_iter()
        .map(|actor| spawn_string_id(&h, actor).unwrap())
        .collect();
    ids.sort();
    assert_eq!(
        ids.len(),
        5,
        "ACTION_RESET_ENCOUNTER respawns channelers: {ids:?}"
    );
    ids.dedup();
    assert_eq!(ids.len(), 5, "no duplicate channeler");
}

#[test]
fn encounter_coverage_kelidan_channeler_combat_speech_bolt_and_mark() {
    for difficulty in [1, 2] {
        let (mut h, _) = kelidan_copy(difficulty);
        let actor = channeler(&mut h, 1);
        let player = h.player();
        let origin = *h.app.world().get::<Position>(actor).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + 5.0,
            ..origin
        });
        watch(&mut h, &[actor]);
        // HARNESS SETUP: direct threat on channeler1.
        crate::creature_aggro::add_threat(h.app.world_mut(), actor, player, 1.0);
        let events = observe(&mut h, 16.0);
        assert_eq!(spoken(&h, actor, CHANNELER, 0), 1, "JustEngagedWith Talk0");
        let bolt = if difficulty == 2 {
            SHADOW_BOLT_H
        } else {
            SHADOW_BOLT
        };
        let bolts = times(&events, CombatLogKind::CastStart, actor, bolt);
        assert!(
            !bolts.is_empty() && bolts[0] <= 5.2,
            "Shadow Bolt0-5s: {bolts:?}"
        );
        let marks: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastStart
                    && event.source == Some(actor.to_bits())
                    && event.spell == Some(MARK_OF_SHADOW)
            })
            .collect();
        assert!(
            !marks.is_empty()
                && marks[0].target == Some(player.to_bits())
                && marks[0].at >= 5.0
                && marks[0].at <= 15.2 + 3.0,
            "Mark of Shadow5-15s at a player without it"
        );
    }
}
