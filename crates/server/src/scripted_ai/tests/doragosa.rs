//! Current Retail LittleWigs2777a9d EchoOfDoragosa.lua and native70448.
use super::*;

const DORAGOSA: u32 = 190609;
const MAP: u32 = 2526;
const BIT: u32 = 3;
const MISSILES: u32 = 373326;

fn doragosa_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(DORAGOSA, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_doragosa_current_native_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (spell, expected) in [
        (MISSILES, (2, 25, 0)),
        (1282251, (32, 6, 1282252)),
        (374343, (3, 18, 374350)),
        (388822, (2, 87, 0)),
    ] {
        let actual: (u32,u32,u32) = conn.query_row("SELECT Effect,ImplicitTarget_0,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0", [spell], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(actual, expected, "current native {spell}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2565",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_doragosa_ordinary_native_missiles_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = doragosa_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, MISSILES, Some(player), false)
            .expect("native373326 admission");
        h.advance(4.0);
        assert_eq!(cast_count(&h, boss, MISSILES), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(MISSILES)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "native missiles damage difficulty{difficulty}"
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
fn encounter_coverage_doragosa_first_missiles_uses_current_retail_clock() {
    let (mut h, boss, _) = doragosa_copy(205);
    h.engage(boss);
    h.advance(11.0);
    assert_eq!(
        cast_count(&h, boss, MISSILES),
        1,
        "current Retail first7s request, not legacy Astral Breath"
    );
}

#[test]
fn encounter_coverage_doragosa_native_astral_blast_children_qualify() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = doragosa_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 1282251, Some(player), false)
            .expect("native1282251");
        // Native parent1282251 has Speed1: its3s cast is followed by flight,
        // not an immediate effect32 hit at cast completion.
        h.advance(3.5);
        assert!(
            !h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(1282252)),
            "native projectile has not arrived"
        );
        h.advance(3.0);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(1282252)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "actual effect32→1282252 damage"
        );
        assert!(h.has_aura(player, 389011), "second native effect32→389011");
        assert!(!h.has_aura(h.players[1].0, 389011));
    }
}

#[test]
fn encounter_coverage_doragosa_ongoing_native_lanes_damage_credit_and_corpse_silence() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = doragosa_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(125.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("DORAGOSA difficulty{difficulty} {attempts:?}");
        // Native3s Blast /1.5s Bomb /4s Vacuum serialize simultaneous deadlines.
        let lanes = [
            (
                MISSILES,
                &[7.0, 17.0, 27.0, 37.0, 50.0, 63.0, 75.0, 91.0, 103.0, 119.0][..],
            ),
            (
                1282251,
                &[9.0, 21.0, 33.5, 45.5, 60.0, 72.0, 88.0, 100.0, 116.0][..],
            ),
            (
                374343,
                &[14.0, 32.0, 48.5, 63.0, 77.0, 91.0, 105.0, 119.0][..],
            ),
            (388822, &[28.0, 56.0, 84.0, 112.0][..]),
        ];
        assert_eq!(
            attempts.len(),
            31,
            "current native lanes only; no canceled6.5s cast"
        );
        for (spell, expected) in lanes {
            let actual: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(actual.len(), expected.len(), "ongoing lane{spell}");
            for (attempt, time) in actual.iter().zip(expected) {
                assert!(attempt.result.is_ok(), "native admission {attempt:?}");
                assert!(
                    (attempt.time - *time).abs() <= 0.15,
                    "native cast-serialized source order {attempt:?} expected{time}"
                );
            }
            assert_eq!(
                cast_count(&h, boss, spell),
                expected.len(),
                "native completions{spell}"
            );
        }
        for spell in [MISSILES, 1282252, 388822] {
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|e| e.kind == CombatLogKind::Damage
                        && e.amount > 0
                        && e.spell_id == Some(spell)
                        && e.source == Some(boss.to_bits())),
                "real scheduled native damage{spell}"
            );
        }
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|s| s.encounter_id == 2565 && s.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|g| g.caster != boss.to_bits())
        );
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
        let ended = h.inbox(0).ends.last().unwrap();
        assert_eq!((ended.encounter_id, ended.success), (2565, true));
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(35.0);
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            before,
            "corpse emits no new requests"
        );
    }
}

#[test]
fn encounter_coverage_doragosa_evade_rearms_first_current_missiles() {
    let (mut h, boss, map) = doragosa_copy(205);
    h.engage(boss);
    h.advance(8.0);
    assert_eq!(cast_count(&h, boss, MISSILES), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    h.engage(boss);
    h.advance(6.8);
    assert_eq!(cast_count(&h, boss, MISSILES), 1);
    h.advance(0.4);
    assert_eq!(cast_count(&h, boss, MISSILES), 2);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}
