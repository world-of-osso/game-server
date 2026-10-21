//! TC a352b1fa boss_broggok.cpp; actual 70448 spells, no historical replacements.
use super::*;
use shared::components::UnitFlags;

const BROGGOK: u32 = 17380;
const MAP: u32 = 542;
const BIT: u32 = 2;
const ENCOUNTER: u32 = 1924;
const SLIME_SPRAY: u32 = 30913;
const POISON_CLOUD: u32 = 30916;

fn broggok_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    bound_boss_at_difficulty(BROGGOK, MAP, BIT, difficulty, None)
}

#[test]
fn encounter_coverage_broggok_native_inputs_qualify() {
    let conn = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("read-only source data");
    let spells = crate::spell_cast::test_harness::world_db().expect("native70448 spells");
    for id in [SLIME_SPRAY, POISON_CLOUD] {
        assert!(spells.store.get(id).is_some(), "native spell{id}");
    }
    for (id, gap) in [
        (30917, "BROGGOK_POISON_BOLT_30917_ABSENT_70448"),
        (26837, "BROGGOK_SUMMON_TRIGGER_26837_ABSENT_70448"),
        (26838, "BROGGOK_DESPAWN_TRIGGER_26838_ABSENT_70448"),
        (30914, "BROGGOK_POISON_AURA_30914_ABSENT_70448"),
        (38462, "BROGGOK_POISON_AURA_38462_ABSENT_70448"),
    ] {
        assert!(spells.store.get(id).is_none(), "{gap}");
    }
    let cloud: (u32, u32) = conn
        .query_row(
            "SELECT Effect,EffectMiscValue_0 FROM spell_effect WHERE SpellID=30916 AND DifficultyID=0 AND EffectIndex=0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        cloud,
        (179, 33896),
        "actual native area trigger, not historical17662 summon"
    );
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    assert_eq!(
        maps.raid_creature_encounters.get(&(MAP, BROGGOK)),
        Some(&BIT)
    );
    assert_eq!(
        conn.query_row(
            "SELECT Name_lang FROM dungeon_encounter WHERE ID=?",
            [ENCOUNTER],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "Broggok"
    );
}

#[test]
#[ignore = "BROGGOK_SOURCE_PORT_PENDING: JustAppeared passive/uninteractible"]
fn encounter_coverage_broggok_appears_dormant_in_both_difficulties() {
    for difficulty in [1, 2] {
        let (mut h, boss, map) = broggok_copy(difficulty);
        assert!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE != 0,
            "TC JustAppeared sets uninteractible in difficulty{difficulty}"
        );
        assert_eq!(
            h.app.world().get::<ReactState>(boss),
            Some(&ReactState::Passive)
        );
        h.advance(20.0);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        assert!(h.inbox(0).starts.is_empty());
        assert!(h.inbox(0).spell_gos.is_empty());
        assert!(h.inbox(1).starts.is_empty());
    }
}

fn watch_broggok(h: &mut Harness, boss: Entity) {
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn talk_count(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&BROGGOK][&group];
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

/// Source release action is called by the final prison wave. This helper tests
/// its boss-side callback only, not the currently unported lever/wave ingress.
fn pull_broggok(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = broggok_copy(difficulty);
    watch_broggok(&mut h, boss);
    dispatch(h.app.world_mut(), boss, Hook::DoAction(1738001));
    h.advance(0.05);
    h.engage(boss);
    (h, boss, map)
}

#[test]
fn encounter_coverage_broggok_source_intro_and_release_action_clock() {
    let (mut h, boss, map) = broggok_copy(1);
    watch_broggok(&mut h, boss);
    let player = h.player();
    let mut distant = *h.app.world().get::<Position>(boss).unwrap();
    distant.x += 100.0;
    h.app.world_mut().entity_mut(player).insert(distant);
    dispatch(h.app.world_mut(), boss, Hook::DoAction(1738000));
    h.advance(1.95);
    assert_eq!(talk_count(&h, boss, 0), 0, "not before source2s");
    h.advance(0.15);
    assert_eq!(talk_count(&h, boss, 0), 1, "one real native intro yell");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    dispatch(h.app.world_mut(), boss, Hook::DoAction(1738001));
    h.advance(0.05);
    assert_eq!(
        h.app.world().get::<ReactState>(boss),
        Some(&ReactState::Aggressive)
    );
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & UnitFlags::NOT_SELECTABLE,
        0
    );
    h.advance(4.9);
    assert_eq!(
        talk_count(&h, boss, 1),
        0,
        "not before source5s release delay"
    );
    h.advance(0.15);
    assert_eq!(talk_count(&h, boss, 1), 1);
    assert!(h.inbox(1).chat.is_empty(), "same actor/copy speech only");
    // Path1381150 has a final authored orientation. Existing MovePath rejects
    // that unsupported shape; do not fake a completion callback or engage.
}

#[test]
fn encounter_coverage_broggok_native_rotation_damage_and_copy_isolation() {
    for difficulty in [1, 2] {
        let (mut h, boss, _) = pull_broggok(difficulty);
        let player = h.player();
        let mut cursor = h.inbox(0).spell_gos.len();
        let mut sprays = Vec::new();
        let mut clouds = Vec::new();
        for tick in 1..=1600 {
            h.advance(0.05);
            let gos = &h.inbox(0).spell_gos;
            for go in &gos[cursor..] {
                if go.caster != boss.to_bits() {
                    continue;
                }
                match go.spell_id {
                    SLIME_SPRAY => sprays.push(tick as f32 * 0.05),
                    POISON_CLOUD => clouds.push(tick as f32 * 0.05),
                    _ => {}
                }
            }
            cursor = gos.len();
        }
        eprintln!("BROGGOK difficulty{difficulty} actual spray={sprays:?} cloud={clouds:?}");
        for (times, first_max, repeat_max) in [(&sprays, 15.0, 15.0), (&clouds, 20.0, 25.0)] {
            assert!(
                times.len() >= 3,
                "native repeated source requests: {times:?}"
            );
            assert!(
                (5.0..=first_max + 0.15).contains(&times[0]),
                "first {times:?}"
            );
            for pair in times.windows(2) {
                let interval = pair[1] - pair[0];
                assert!(
                    (9.9..=repeat_max + 0.15).contains(&interval),
                    "repeat {times:?}"
                );
            }
        }
        assert!(
            h.inbox(0).log.iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(SLIME_SPRAY)
            }),
            "ordinary native30913 cone damage"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == ENCOUNTER && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        assert_eq!(
            cast_count(&h, boss, 30917),
            0,
            "absent bolt has no substitute"
        );
    }
}

#[test]
fn encounter_coverage_broggok_reset_death_and_corpse_silence() {
    let (mut h, boss, map) = pull_broggok(2);
    h.advance(21.0);
    assert!(cast_count(&h, boss, SLIME_SPRAY) > 0);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    h.engage(boss);
    let before = cast_count(&h, boss, SLIME_SPRAY);
    h.advance(4.8);
    assert_eq!(
        cast_count(&h, boss, SLIME_SPRAY),
        before,
        "reset source earliest5s"
    );
    h.advance(16.2);
    assert!(cast_count(&h, boss, SLIME_SPRAY) > before);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
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
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    let casts = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits())
        .count();
    h.advance(30.0);
    assert_eq!(
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == boss.to_bits())
            .count(),
        casts,
        "corpse cannot keep casting"
    );
}
