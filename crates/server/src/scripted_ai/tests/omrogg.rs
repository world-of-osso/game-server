//! TC a352b1fa Shattered Halls Warbringer O'mrogg16809 (boss_warbringer_omrogg.cpp).
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

const OMROGG: u32 = 16809;
const LEFT_HEAD: u32 = 19523;
const RIGHT_HEAD: u32 = 19524;
const SHATTERED_HALLS: u32 = 540;
/// TC DATA_OMROGG: installed DB2 encounter1937, completion bit2.
const OMROGG_BIT: u32 = 2;
const OMROGG_ENCOUNTER: u32 = 1937;
const FEAR: u32 = 30584;
const THUNDERCLAP: u32 = 30633;
const BEATDOWN: u32 = 30618;
const BURNING_MAUL: u32 = 30598;
const BURNING_MAUL_H: u32 = 36056;
const BURNING_MAUL_HIT: u32 = 36057;
const BURNING_MAUL_PERIODIC: u32 = 30620;
const BLAST_WAVE: u32 = 30600;
/// `creature_equip_template` 16809: ID1 the spawn's weapon, ID2 EQUIP_ID_BURNING_MAUL.
const ORIGINAL_WEAPON: u32 = 29484;
const BURNING_MAUL_WEAPON: u32 = 29479;

/// A Shattered Halls copy filled by production copy fill from the real map540 rows,
/// with authentic ground and eye tables.
fn omrogg_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        OMROGG,
        SHATTERED_HALLS,
        OMROGG_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
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

/// Burning Maul36057 Damage lines by `caster` logged since event index `start`.
fn maul_hits(h: &Harness, caster: Entity, start: usize) -> usize {
    h.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(BURNING_MAUL_HIT)
        })
        .count()
}

/// The first player stands `offset` yards east of O'mrogg.
fn stand_beside(h: &mut Harness, boss: Entity, offset: f32) {
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + offset,
        ..origin
    });
}

fn weapon(h: &Harness, unit: Entity) -> Option<u32> {
    h.app
        .world()
        .get::<shared::components::EquipmentAppearance>(unit)
        .and_then(|equipment| equipment.entries.first())
        .and_then(|entry| entry.item_id)
}

#[test]
fn encounter_coverage_omrogg_native_inputs_qualify() {
    for difficulty in [1, 2] {
        let (mut h, map) = omrogg_copy(difficulty);
        assert_eq!(
            h.app
                .world()
                .resource::<MapStore>()
                .instance_scripts
                .get(&SHATTERED_HALLS)
                .map(String::as_str),
            Some("instance_shattered_halls")
        );
        for entry in [OMROGG, LEFT_HEAD, RIGHT_HEAD] {
            assert_eq!(
                h.creatures_of(entry).len(),
                1,
                "TDB spawnDifficulties1,2: {entry} in difficulty{difficulty}"
            );
        }
        let boss = h.creatures_of(OMROGG)[0];
        assert_eq!(
            weapon(&h, boss),
            Some(ORIGINAL_WEAPON),
            "spawn equipment_id1"
        );
        let equipment = &h.app.world().resource::<WorldData>().equipment[&OMROGG];
        let maul = equipment[&2]
            .entries
            .first()
            .and_then(|entry| entry.item_id);
        assert_eq!(maul, Some(BURNING_MAUL_WEAPON), "EQUIP_ID_BURNING_MAUL");

        let texts = &h.app.world().resource::<CreatureTexts>().groups;
        assert_eq!(texts[&OMROGG][&0].len(), 1, "EMOTE_ROAR");
        assert_eq!(texts[&OMROGG][&0][0].chat_type, 16, "MonsterEmote");
        let left: Vec<_> = (0..=11).map(|g| texts[&LEFT_HEAD][&g].len()).collect();
        assert_eq!(left, [1; 12], "left head groups0-11");
        let right: Vec<_> = (0..=10).map(|g| texts[&RIGHT_HEAD][&g].len()).collect();
        assert_eq!(right, [1; 11], "right head groups0-10");

        let spells = h.app.world().resource::<SpellStore>();
        // OMROGG_BURNING_MAUL_30620_ABSENT_70448: the periodic trigger has no SpellInfo.
        assert!(spells.get(BURNING_MAUL_PERIODIC).is_none());
        for spell in [
            FEAR,
            THUNDERCLAP,
            BEATDOWN,
            BURNING_MAUL,
            BURNING_MAUL_H,
            BURNING_MAUL_HIT,
            BLAST_WAVE,
        ] {
            assert!(spells.get(spell).is_some(), "spell{spell}");
        }

        // HARNESS SETUP: direct threat, not a walked pull.
        h.engage(boss);
        h.advance(0.5);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, OMROGG_BIT),
            Some(EncounterState::InProgress)
        );
        assert!(
            h.inbox(0).starts.iter().any(|start| {
                start.encounter_id == OMROGG_ENCOUNTER && start.difficulty_id == difficulty
            }),
            "bit2 -> DungeonEncounter1937 in difficulty{difficulty}"
        );
        assert!(h.inbox(1).starts.is_empty(), "other copy isolated");
    }
}

#[test]
fn encounter_coverage_omrogg_native_spell_admissions_qualify() {
    for difficulty in [1, 2] {
        let (mut h, _) = omrogg_copy(difficulty);
        h.app.init_resource::<ObservedCombatEvents>();
        let boss = h.creatures_of(OMROGG)[0];
        stand_beside(&mut h, boss, 2.0);
        watch(&mut h, &[boss]);
        let maul = if difficulty == 2 {
            BURNING_MAUL_H
        } else {
            BURNING_MAUL
        };
        // Thunderclap and Beatdown share category1152 "Creature Special 1":
        // Beatdown locks it8s, Thunderclap3s, from completion of their1s casts.
        // Each cast waits out the prior lock.
        for spell in [FEAR, BEATDOWN, THUNDERCLAP, BLAST_WAVE, maul] {
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, None, false)
                .unwrap_or_else(|error| panic!("ordinary self-area {spell}: {error:?}"));
            h.advance(9.5);
            eprintln!(
                "OMROGG difficulty{difficulty} spell{spell} casts={}",
                observed_casts(&h, boss, spell)
            );
            assert_eq!(observed_casts(&h, boss, spell), 1, "spell{spell} completes");
        }
        assert!(h.has_aura(boss, maul), "Burning Maul self aura");

        // The aura's PROC_TRIGGER_SPELL default: melee hits trigger Burning Maul36057,
        // a triggered spell logged by its Damage line, not a CastSuccess.
        // HARNESS SETUP: direct threat so O'mrogg swings at the adjacent player.
        h.engage(boss);
        let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
        h.advance(6.0);
        let hits = maul_hits(&h, boss, start);
        eprintln!("OMROGG difficulty{difficulty} maul hits={hits}");
        assert!(hits > 0, "default proc of the Burning Maul aura");
    }
}

/// A cast or line at simulated seconds after observation started.
struct Timed {
    at: f32,
    kind: CombatLogKind,
    source: Option<u64>,
    spell: Option<u32>,
}

/// A head line: `(at, speaker entry, text group)`.
type Line = (f32, u32, u8);

/// Advance `secs` in50ms ticks, timestamping newly logged combat events and
/// head/boss lines heard by the first player; `on_tick` sees the simulated time.
fn observe_with(
    h: &mut Harness,
    secs: f32,
    mut on_tick: impl FnMut(&mut Harness, f32),
) -> (Vec<Timed>, Vec<Line>) {
    h.app.init_resource::<ObservedCombatEvents>();
    let mut cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
    let mut chat_cursor = h.inbox(0).chat.len();
    let (mut timed, mut lines) = (Vec::new(), Vec::new());
    for tick in 1..=(secs / 0.05).round() as usize {
        h.advance(0.05);
        let at = tick as f32 * 0.05;
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        timed.extend(events[cursor..].iter().map(|event| Timed {
            at,
            kind: event.kind.clone(),
            source: event.source,
            spell: event.spell_id,
        }));
        cursor = events.len();
        let heard: Vec<_> = h.inbox(0).chat[chat_cursor..].to_vec();
        chat_cursor = h.inbox(0).chat.len();
        lines.extend(heard.iter().filter_map(|line| line_of(h, at, line)));
        on_tick(h, at);
    }
    (timed, lines)
}

fn observe(h: &mut Harness, secs: f32) -> (Vec<Timed>, Vec<Line>) {
    observe_with(h, secs, |_, _| {})
}

/// The `(entry, group)` of an O'mrogg or head template text matching `line`.
fn line_of(h: &Harness, at: f32, line: &ChatMessage) -> Option<Line> {
    let texts = &h.app.world().resource::<CreatureTexts>().groups;
    [OMROGG, LEFT_HEAD, RIGHT_HEAD]
        .into_iter()
        .find_map(|entry| {
            texts[&entry].iter().find_map(|(&group, entries)| {
                entries
                    .iter()
                    .any(|text| line.content == text.male_text || line.content == text.female_text)
                    .then_some((at, entry, group))
            })
        })
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

const L: u32 = LEFT_HEAD;
const R: u32 = RIGHT_HEAD;
/// TC npc_omrogg_heads DoAction sequences: `(speaker, group, seconds after the first)`.
const AGGRO: [&[(u32, u8, f32)]; 3] = [
    &[(L, 0, 0.0), (R, 0, 3.0), (L, 1, 6.0)],
    &[(L, 2, 0.0), (R, 1, 3.0)],
    &[(L, 3, 0.0), (R, 2, 3.0)],
];
const ATTACK: [&[(u32, u8, f32)]; 4] = [
    &[(L, 4, 0.0), (R, 3, 3.0), (L, 5, 6.0)],
    &[(L, 6, 0.0), (R, 4, 3.0)],
    &[(L, 7, 0.0), (R, 5, 3.0), (L, 8, 6.0)],
    &[(R, 6, 0.0), (L, 9, 2.0), (R, 7, 4.0)],
];
const SLAY: [&[(u32, u8, f32)]; 2] = [&[(R, 8, 0.0)], &[(L, 10, 0.0), (R, 9, 3.0)]];
const DEATH: &[(u32, u8, f32)] = &[(L, 11, 0.0), (R, 10, 3.0)];
/// One scheduler tick of slack around each authored delay.
const LINE_SLACK: f32 = 0.15;

/// `lines` are exactly one of `variants`, at its authored offsets; returns its index.
fn assert_sequence(lines: &[Line], variants: &[&[(u32, u8, f32)]]) -> usize {
    let first = lines.first().expect("a head sequence");
    let index = variants
        .iter()
        .position(|variant| (variant[0].0, variant[0].1) == (first.1, first.2))
        .unwrap_or_else(|| panic!("{first:?} opens no authored sequence"));
    let variant = variants[index];
    let actual: Vec<_> = lines.iter().map(|line| (line.1, line.2)).collect();
    let expected: Vec<_> = variant.iter().map(|step| (step.0, step.1)).collect();
    assert_eq!(actual, expected, "sequence {index} speakers and groups");
    for (line, step) in lines.iter().zip(variant) {
        let offset = line.0 - first.0;
        assert!(
            (offset - step.2).abs() <= LINE_SLACK,
            "{step:?} at +{offset}s"
        );
    }
    index
}

fn heads(h: &mut Harness) -> [Entity; 2] {
    [h.creatures_of(LEFT_HEAD)[0], h.creatures_of(RIGHT_HEAD)[0]]
}

/// O'mrogg with the first player adjacent and both heads audible; nothing engaged.
fn listening(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, map) = omrogg_copy(difficulty);
    let boss = h.creatures_of(OMROGG)[0];
    stand_beside(&mut h, boss, 2.0);
    let [left, right] = heads(&mut h);
    watch(&mut h, &[boss, left, right]);
    (h, boss, map)
}

/// Fixture RNG seed for O'mrogg's EventMap, consumed at JustEngagedWith: category1152
/// can legitimately drop a Beatdown/Thunderclap roll inside the other's lock.
fn seed_rotation(h: &mut Harness, boss: Entity) {
    h.app.world_mut().entity_mut(boss).insert(
        crate::scripted_ai::tests::cast_admission::GroundCombatSeed(0),
    );
}

fn threat_of(h: &Harness, boss: Entity) -> Vec<(u64, f32)> {
    h.app
        .world()
        .get::<shared::components::ThreatTable>(boss)
        .map(|table| {
            table
                .entries()
                .iter()
                .map(|entry| (entry.entity, entry.threat))
                .collect()
        })
        .unwrap_or_default()
}

fn current_victim(h: &mut Harness, boss: Entity) -> Option<Entity> {
    crate::creature_aggro::current_victim(h.app.world_mut(), boss)
}

#[test]
#[ignore = "RED: O'mrogg script not ported"]
fn encounter_coverage_omrogg_aggro_dialogue_from_the_heads() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = listening(difficulty);
        // HARNESS SETUP: direct threat, not a walked pull.
        h.engage(boss);
        let (_, lines) = observe(&mut h, 8.0);
        eprintln!("OMROGG difficulty{difficulty} aggro lines={lines:?}");
        assert!(lines.first().is_some_and(|line| line.0 <= 0.6), "{lines:?}");
        assert_sequence(&lines, &AGGRO);
    }
}

#[test]
#[ignore = "RED: O'mrogg script not ported"]
fn encounter_coverage_omrogg_rotation_beatdown_lines_and_burning_maul_roar() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = listening(difficulty);
        seed_rotation(&mut h, boss);
        h.engage(boss);
        let (events, lines) = observe(&mut h, 64.0);
        let success = |spell| times(&events, CombatLogKind::CastSuccess, boss, spell);
        let (fear, clap, beat, maul) = (
            success(FEAR),
            success(THUNDERCLAP),
            success(BEATDOWN),
            success(BURNING_MAUL),
        );
        eprintln!(
            "OMROGG difficulty{difficulty} fear={fear:?} clap={clap:?} beat={beat:?} maul={maul:?}"
        );
        // First timers plus at most one competing1s cast and a tick.
        let window = |casts: &[f32], min: f32, max: f32| {
            casts
                .first()
                .is_some_and(|&at| at >= min && at <= max + 2.1)
        };
        assert!(window(&fear, 20.0, 30.0), "Fear20-30s: {fear:?}");
        assert!(
            window(&clap, 15.0, 26.0),
            "Thunderclap15-25s +1s cast: {clap:?}"
        );
        assert!(
            window(&beat, 25.0, 31.0),
            "Beatdown25-30s +1s cast: {beat:?}"
        );
        assert!(window(&maul, 50.0, 60.0), "Burning Maul50-60s: {maul:?}");
        assert!(
            success(BURNING_MAUL_H).is_empty(),
            "the script casts30598 in both difficulties"
        );

        let roars: Vec<_> = lines
            .iter()
            .filter(|line| line.1 == OMROGG && line.2 == 0)
            .map(|line| line.0)
            .collect();
        assert_eq!(roars, maul, "OnSpellCast(30598) EMOTE_ROAR");

        let after_first_beatdown: Vec<_> = lines
            .iter()
            .filter(|line| line.1 != OMROGG && line.0 >= beat[0] && line.0 < beat[0] + 7.0)
            .copied()
            .collect();
        eprintln!("OMROGG difficulty{difficulty} beatdown lines={after_first_beatdown:?}");
        assert!(
            (after_first_beatdown[0].0 - beat[0]).abs() <= LINE_SLACK,
            "OnSpellCast(30618) starts a head sequence"
        );
        assert_sequence(&after_first_beatdown, &ATTACK);
    }
}

#[test]
#[ignore = "RED: Beatdown AfterCast ResetAllThreat not ported"]
fn encounter_coverage_omrogg_beatdown_zeroes_threat_and_keeps_references() {
    let (mut h, boss, map) = listening(1);
    let tank = h.player();
    let other = h.players[1].0;
    // HARNESS SETUP: the second player joins this copy beside the tank.
    let tank_position = *h.app.world().get::<Position>(tank).unwrap();
    h.app
        .world_mut()
        .entity_mut(other)
        .insert((map, tank_position));
    seed_rotation(&mut h, boss);
    // HARNESS SETUP: direct threat, the tank far ahead of the second player.
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, tank, 1000.0);
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, other, 100.0);
    h.advance(1.1);
    assert_eq!(current_victim(&mut h, boss), Some(tank));

    let mut after_beatdown = None;
    let (events, _) = observe_with(&mut h, 32.0, |h, at| {
        if observed_casts(h, boss, BEATDOWN) > 0 && after_beatdown.is_none() {
            after_beatdown = Some((at, threat_of(h, boss)));
        }
    });
    let beat = times(&events, CombatLogKind::CastSuccess, boss, BEATDOWN);
    let (at, threat) = after_beatdown.expect("Beatdown within25-31s");
    eprintln!("OMROGG beatdown at {at} casts={beat:?} threat={threat:?}");
    // TC ThreatManager::ResetAllThreat (ThreatManager.cpp:527-531): ScaleThreat(0)
    // on every reference; references stay listed.
    assert_eq!(threat.len(), 2, "both references kept");
    assert!(
        threat.iter().all(|&(_, amount)| amount == 0.0),
        "{threat:?}"
    );
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, OMROGG_BIT),
        Some(EncounterState::InProgress),
        "the encounter continues"
    );

    // HARNESS SETUP: the second player is first to threaten after the reset.
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, other, 1.0);
    h.advance(1.1);
    assert_eq!(
        current_victim(&mut h, boss),
        Some(other),
        "1 threat beats the tank's zeroed1000 (ReselectVictim 110%)"
    );
}

#[test]
#[ignore = "RED: Burning Maul AuraScript not ported"]
fn encounter_coverage_omrogg_burning_maul_swaps_weapon_and_procs_blast_wave() {
    let (mut h, boss, _) = listening(1);
    seed_rotation(&mut h, boss);
    h.engage(boss);
    let mut weapons = Vec::new();
    // Two Burning Maul windows: 50-60s, then 60-70s later, 18s each.
    let (events, _) = observe_with(&mut h, 150.0, |h, at| {
        let aura = h.has_aura(boss, BURNING_MAUL);
        weapons.push((at, aura, weapon(h, boss)));
    });
    let mauls = times(&events, CombatLogKind::CastSuccess, boss, BURNING_MAUL);
    let hits = times(&events, CombatLogKind::Damage, boss, BURNING_MAUL_HIT);
    let waves = times(&events, CombatLogKind::CastStart, boss, BLAST_WAVE);
    eprintln!("OMROGG mauls={mauls:?} hits={hits:?} waves={waves:?}");
    assert_eq!(mauls.len(), 2, "two Burning Maul casts in150s");
    for &(at, aura, weapon) in &weapons {
        let expected = if aura {
            BURNING_MAUL_WEAPON
        } else {
            ORIGINAL_WEAPON
        };
        assert_eq!(
            weapon,
            Some(expected),
            "LoadEquipment while aura={aura} at {at}"
        );
    }
    assert!(weapons.iter().any(|&(_, aura, _)| aura), "aura observed");
    // OnProc: roll_chance(30) Blast Wave on a default36057 proc tick.
    for wave in &waves {
        assert!(hits.contains(wave), "Blast Wave {wave} rides a36057 proc");
    }
    assert!(
        hits.len() >= 12,
        "enough procs that no Blast Wave has p<=0.7^12: {}",
        hits.len()
    );
    assert!(!waves.is_empty(), "30% Blast Wave on proc");
}

#[test]
#[ignore = "RED: O'mrogg script not ported"]
fn encounter_coverage_omrogg_slay_lines_from_the_heads() {
    let (mut h, boss, _) = listening(1);
    super::omor_slay::install_player_death_transition(&mut h);
    h.engage(boss);
    h.advance(8.0);
    let player = h.player();
    // HARNESS SETUP: production Unit::Kill attributes the player's death to O'mrogg.
    crate::combat::kill_unit(h.app.world_mut(), boss, player);
    let (_, lines) = observe(&mut h, 4.0);
    eprintln!("OMROGG slay lines={lines:?}");
    assert!(lines.first().is_some_and(|line| line.0 <= 0.2), "{lines:?}");
    assert_sequence(&lines, &SLAY);
}

#[test]
#[ignore = "RED: O'mrogg script not ported"]
fn encounter_coverage_omrogg_death_lines_then_heads_despawn() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = listening(difficulty);
        h.engage(boss);
        h.advance(8.0);
        let [left, right] = heads(&mut h);
        let mut alive = Vec::new();
        h.hit_pct(boss, 200.0);
        let (_, lines) = observe_with(&mut h, 8.0, |h, at| {
            let exists = |unit| h.app.world().get_entity(unit).is_ok();
            alive.push((at, exists(left), exists(right)));
        });
        eprintln!("OMROGG difficulty{difficulty} death lines={lines:?}");
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, OMROGG_BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == OMROGG_ENCOUNTER && end.success)
                .count(),
            1
        );
        assert!(lines.first().is_some_and(|line| line.0 <= 0.2), "{lines:?}");
        assert_sequence(&lines, &[DEATH]);
        // DespawnOrUnsummon(4s) for both heads after the right head's line.
        let despawn_at = lines[1].0 + 4.0;
        for &(at, left_alive, right_alive) in &alive {
            if at < despawn_at - LINE_SLACK {
                assert!(left_alive && right_alive, "heads present at {at}");
            } else if at > despawn_at + LINE_SLACK {
                assert!(!left_alive && !right_alive, "heads despawned at {at}");
            }
        }
    }
}
