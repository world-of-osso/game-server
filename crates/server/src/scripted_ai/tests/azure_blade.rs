//! LittleWigs2777a9d Azureblade.lua + native70448; partial Lua-clock port.
use super::*;

const AZUREBLADE: u32 = 186739;
const MAP: u32 = 2515;
/// Preserve installed Azure saved ordinal1; DB2 encounter2585 completion bit3.
const ORDINAL: u32 = 1;
const ARCANE_CLEAVE: u32 = 372222;
const IMAGE: u32 = 384223;
const ANCIENT_ORB: u32 = 385578;

fn azureblade_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(AZUREBLADE, MAP, ORDINAL, difficulty, None);
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
fn encounter_coverage_azureblade_native_inputs_qualify() {
    let all = crate::spell_cast::test_harness::world_db().expect("native70448");
    for id in [ARCANE_CLEAVE, IMAGE, ANCIENT_ORB, 373658, 373582, 384132] {
        assert!(all.store.get(id).is_some(), "native spell{id}");
    }
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let cleave: (u32, u32, u32) = conn.query_row(
        "SELECT Effect,ImplicitTarget_0,EffectRadiusIndex_0 FROM spell_effect WHERE SpellID=372222 AND DifficultyID=0 AND EffectIndex=0",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(cleave, (2, 104, 13), "native destination-facing cone");
    let image: (u32,u32,u32) = conn.query_row(
        "SELECT Effect,EffectMiscValue_0,EffectMiscValue_1 FROM spell_effect WHERE SpellID=373582 AND DifficultyID=0 AND EffectIndex=0",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(image, (28, 190187, 64), "native image chain actor/property");
}

#[test]
fn encounter_coverage_azureblade_ordinary_cleave_damage_qualifies() {
    let (mut h, boss, _) = azureblade_copy(205);
    let player = h.player();
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, ARCANE_CLEAVE, Some(player), false)
        .expect("actual ordinary372222 admission");
    h.advance(2.1);
    assert_eq!(cast_count(&h, boss, ARCANE_CLEAVE), 1);
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(ARCANE_CLEAVE)
        }),
        "actual native cone damages intended player"
    );
    assert_eq!(cast_count(&h, boss, IMAGE), 0);
    assert_eq!(cast_count(&h, boss, ANCIENT_ORB), 0);
}

#[test]
#[ignore = "AZUREBLADE_LUA_CLOCK_PORT_PENDING"]
fn encounter_coverage_azureblade_first_cleave_uses_lua_clock() {
    let (mut h, boss, _) = azureblade_copy(205);
    h.engage(boss);
    h.advance(8.0);
    assert_eq!(
        cast_count(&h, boss, ARCANE_CLEAVE),
        1,
        "Lua OnEngage5.2s request + actual native2s cast"
    );
}

#[test]
fn encounter_coverage_azureblade_native_bounded_rotation_and_image_chain() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = azureblade_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(45.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("AZUREBLADE difficulty{difficulty} actual attempts={attempts:?}");
        for (spell, first, repeat) in [
            (IMAGE, 3.3, 14.6),
            (ARCANE_CLEAVE, 5.2, 13.4),
            (ANCIENT_ORB, 10.6, 15.8),
        ] {
            let casts: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert_eq!(casts.len(), 2, "Lua first-phase count2, spell{spell}");
            assert!(
                casts.iter().all(|attempt| attempt.result.is_ok()),
                "ordinary native admission{spell}: {casts:?}"
            );
            assert!(
                (casts[0].time - first).abs() <= 0.1,
                "first source request{spell}: {casts:?}"
            );
            let due = first + repeat;
            let expected_second = if spell == ARCANE_CLEAVE {
                let image_cast = h
                    .app
                    .world()
                    .resource::<SpellStore>()
                    .get(IMAGE)
                    .unwrap()
                    .cast_time
                    .unwrap()
                    .base_ms as f32
                    / 1000.0;
                due.max(3.3 + 14.6 + image_cast)
            } else {
                due
            };
            assert!(
                (casts[1].time - expected_second).abs() <= 0.1,
                "source deadline/native casting wait{spell}: {casts:?}"
            );
            assert_eq!(
                cast_count(&h, boss, spell),
                2,
                "actual native completions{spell}"
            );
        }
        let images = h.creatures_of(190187);
        assert_eq!(
            images.len(),
            2,
            "ordinary384223→373658→373582 actual summons"
        );
        for image in images {
            assert_eq!(h.app.world().get::<InMap>(image), Some(&map));
            assert_eq!(
                h.app
                    .world()
                    .get::<crate::zones::TempSummon>(image)
                    .unwrap()
                    .summoner,
                boss
            );
        }
        assert_eq!(
            cast_count(&h, boss, 384132),
            0,
            "energy/phase mechanic is a named unsourced gap"
        );
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 2585 && start.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        h.hit_pct(boss, 200.0);
        h.advance(0.1);
        assert!(
            h.creatures_of(190187).is_empty(),
            "generic BossAi death cleanup"
        );
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, ORDINAL),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << ORDINAL,
            "preserved Azure saved ordinal, not DB2bit3"
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 2585 && end.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = cast_count(&h, boss, ARCANE_CLEAVE);
        h.advance(30.0);
        assert_eq!(
            cast_count(&h, boss, ARCANE_CLEAVE),
            before,
            "corpse silence"
        );
    }
}

#[test]
fn encounter_coverage_azureblade_evade_rearms_bounded_requests_without_credit() {
    let (mut h, boss, map) = azureblade_copy(205);
    h.engage(boss);
    h.advance(9.0);
    assert_eq!(h.creatures_of(190187).len(), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert!(h.creatures_of(190187).is_empty());
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, ORDINAL),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    let before = cast_count(&h, boss, ARCANE_CLEAVE);
    h.engage(boss);
    h.advance(5.0);
    assert_eq!(
        cast_count(&h, boss, ARCANE_CLEAVE),
        before,
        "no invented early cleave on reset"
    );
    h.advance(3.0);
    assert_eq!(cast_count(&h, boss, ARCANE_CLEAVE), before + 1);
    assert_eq!(
        h.creatures_of(190187).len(),
        1,
        "fresh native image on repull"
    );
}
