//! Current Retail LittleWigs2777a9d Kokia.lua + actual70448 native targets/effects.
use super::*;

const KOKIA: u32 = 189232;
const MAP: u32 = 2521;
const BIT: u32 = 0;
const SEARING_BLOWS: u32 = 372858;

fn kokia_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(KOKIA, MAP, BIT, difficulty, None);
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
fn encounter_coverage_kokia_current_retail_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (spell, expected) in [
        (372864, (3, 0, 0)),
        (372110, (3, 0, 372107)),
        (SEARING_BLOWS, (6, 23, 372859)),
    ] {
        let actual:(u32,u32,u32)=conn.query_row("SELECT Effect,EffectAura,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0",[spell],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(actual, expected, "current native effect/child{spell}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2606",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_kokia_native_searing_periodic_weapon_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = kokia_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            boss,
            SEARING_BLOWS,
            Some(player),
            false,
        )
        .expect("ordinary native372858 admission");
        h.advance(3.5);
        assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(372859)),
            "real native aura23→372859 weapon damage in difficulty{difficulty}"
        );
        assert!(!h.has_aura(h.players[1].0, SEARING_BLOWS));
    }
}

#[test]
#[ignore = "KOKIA_CURRENT_RETAIL_PORT_PENDING"]
fn encounter_coverage_kokia_first_searing_blows_follows_current_retail_clock() {
    let (mut h, boss, _) = kokia_copy(205);
    h.engage(boss);
    h.advance(32.0);
    assert_eq!(
        cast_count(&h, boss, SEARING_BLOWS),
        1,
        "current Retail28s ability request, not legacy21.6s"
    );
}

#[test]
fn encounter_coverage_kokia_current_lanes_native_damage_and_isolated_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = kokia_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(140.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("KOKIA difficulty{difficulty} actual native requests={attempts:?}");
        let lanes = [
            (372864, &[8.0, 48.0, 88.0, 128.0][..]),
            (372107, &[19.0, 39.0, 58.0, 78.0, 97.0, 117.0, 136.0][..]),
            (SEARING_BLOWS, &[28.0, 68.0, 108.0][..]),
        ];
        assert_eq!(
            attempts.len(),
            14,
            "only current native source lanes, no legacy substitutes"
        );
        for (spell, expected) in lanes {
            let casts: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert_eq!(
                casts.len(),
                expected.len(),
                "ongoing native source lane{spell}"
            );
            for (attempt, time) in casts.iter().zip(expected) {
                assert!(
                    attempt.result.is_ok(),
                    "ordinary native admission{attempt:?}"
                );
                assert!(
                    (attempt.time - *time).abs() <= 0.1,
                    "currentRetail source-order lane{spell}: {attempt:?} expected{time}"
                );
            }
            assert_eq!(
                cast_count(&h, boss, spell),
                expected.len(),
                "actual native completions{spell}"
            );
        }
        let player = h.player();
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(372859)),
            "real scheduled aura23 weapon damage"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 2606 && start.difficulty_id == difficulty)
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
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT,
            "Kokia only, no Melidrussa bit"
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 2606 && end.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = cast_count(&h, boss, SEARING_BLOWS)
            + cast_count(&h, boss, 372107)
            + cast_count(&h, boss, 372864);
        h.advance(45.0);
        assert_eq!(
            cast_count(&h, boss, SEARING_BLOWS)
                + cast_count(&h, boss, 372107)
                + cast_count(&h, boss, 372864),
            before,
            "corpse silence"
        );
    }
}

#[test]
fn encounter_coverage_kokia_evade_rearms_current_source_lanes() {
    let (mut h, boss, map) = kokia_copy(205);
    h.engage(boss);
    h.advance(32.0);
    assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 1);
    assert_eq!(cast_count(&h, boss, 372107), 1);
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
    h.advance(27.8);
    assert_eq!(
        cast_count(&h, boss, SEARING_BLOWS),
        1,
        "not before currentRetail28s"
    );
    h.advance(4.0);
    assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 2);
    h.advance(11.0);
    assert_eq!(
        cast_count(&h, boss, 372107),
        3,
        "reset resumes19 then20 source durations/native2.5s casts"
    );
}
