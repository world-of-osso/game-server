//! LittleWigs2777a9d Umbrelskul.lua + native70448; partial Lua-clock combat.
use super::*;

const UMBRELSKUL: u32 = 186738;
const MAP: u32 = 2515;
const ORDINAL: u32 = 3;
const DRAGON_STRIKE: u32 = 384978;
const CRYSTALLINE_ROAR: u32 = 384699;

fn umbrelskul_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(UMBRELSKUL, MAP, ORDINAL, difficulty, None);
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
fn encounter_coverage_umbrelskul_native_inputs_qualify() {
    let all = crate::spell_cast::test_harness::world_db().expect("native70448");
    for id in [
        DRAGON_STRIKE,
        CRYSTALLINE_ROAR,
        384696,
        385075,
        385399,
        388804,
        386746,
    ] {
        assert!(all.store.get(id).is_some(), "native spell{id}");
    }
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let strike:Vec<(u32,u32,u32)>=conn.prepare(
        "SELECT EffectIndex,Effect,EffectAura FROM spell_effect WHERE SpellID=384978 AND DifficultyID=0 ORDER BY EffectIndex").unwrap()
        .query_map([],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap().map(Result::unwrap).collect();
    assert_eq!(strike, [(0, 2, 0), (1, 6, 3), (2, 138, 0)]);
    let roar:(u32,u32,u32)=conn.query_row(
        "SELECT Effect,ImplicitTarget_0,EffectRadiusIndex_0 FROM spell_effect WHERE SpellID=384699 AND DifficultyID=0 AND EffectIndex=1",[],
        |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(roar.0, 2);
    assert_eq!(
        roar.1, 129,
        "native caster-facing line, not fabricated area"
    );
}

#[test]
fn encounter_coverage_umbrelskul_ordinary_strike_damage_and_periodic_qualify() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    let player = h.player();
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, DRAGON_STRIKE, Some(player), false)
        .expect("ordinary actual384978 admission");
    h.advance(4.6);
    assert_eq!(cast_count(&h, boss, DRAGON_STRIKE), 1);
    assert!(
        h.has_aura(player, DRAGON_STRIKE),
        "native damage-over-time aura"
    );
    let damage: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(DRAGON_STRIKE)
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
        })
        .collect();
    assert!(
        damage.len() >= 2,
        "actual direct damage plus periodic tick(s): {damage:?}"
    );
    assert!(
        !h.has_aura(h.players[1].0, DRAGON_STRIKE),
        "other copy excluded"
    );
}

#[test]
fn encounter_coverage_umbrelskul_unleashed_destruction_and_credit() {
    let (mut h, boss, map) = umbrelskul_copy(205);
    h.engage(boss);
    h.advance(54.7);
    assert_eq!(cast_count(&h, boss, 385399), 0);
    h.advance(8.0);
    assert_eq!(
        cast_count(&h, boss, 385399),
        1,
        "native first Unleashed Destruction"
    );
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(385399)
                && event.source == Some(boss.to_bits())
                && event.target == Some(h.player().to_bits())),
        "native area damage"
    );
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, ORDINAL),
        Some(EncounterState::Done)
    );
    assert_ne!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & (1 << 2),
        0
    );
}

#[test]
fn encounter_coverage_umbrelskul_first_strike_uses_lua_clock() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    h.engage(boss);
    h.advance(10.1);
    assert_eq!(
        cast_count(&h, boss, DRAGON_STRIKE),
        1,
        "Lua7.4s first request/native2.5s cast"
    );
}

fn native_cast_secs(h: &Harness, spell: u32) -> f32 {
    h.app
        .world()
        .resource::<SpellStore>()
        .get(spell)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .cast_time
        .unwrap()
        .base_ms as f32
        / 1000.0
}

#[test]
fn encounter_coverage_umbrelskul_ordinary_roar_native_line_damage_qualifies() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    let player = h.player();
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        boss,
        CRYSTALLINE_ROAR,
        Some(player),
        false,
    )
    .expect("ordinary native384699 admission");
    h.advance(5.1);
    assert_eq!(cast_count(&h, boss, CRYSTALLINE_ROAR), 1);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(CRYSTALLINE_ROAR)
                && event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())),
        "actual native caster-line129 damage, no invented radius"
    );
}

#[test]
fn encounter_coverage_umbrelskul_native_lua_rotation_and_isolated_credit() {
    for (difficulty, strike_repeat, roar_repeat, seconds) in [
        (1, 8.4, 23.1, 65.0),
        (205, 8.4, 23.1, 65.0),
        (23, 15.3, 105.6, 125.0),
    ] {
        let (mut h, boss, map) = umbrelskul_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        let mut cursor = h.inbox(0).spell_gos.len();
        let mut completions = Vec::new();
        for tick in 1..=(seconds / 0.05_f32).round() as usize {
            h.advance(0.05);
            let gos = &h.inbox(0).spell_gos;
            completions.extend(
                gos[cursor..]
                    .iter()
                    .filter(|go| go.caster == boss.to_bits())
                    .map(|go| (go.spell_id, tick as f32 * 0.05)),
            );
            cursor = gos.len();
        }
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!(
            "UMBRELSKUL difficulty{difficulty} requests={attempts:?} completions={completions:?}"
        );
        let strikes: Vec<_> = attempts
            .iter()
            .filter(|attempt| attempt.spell == DRAGON_STRIKE)
            .collect();
        let roars: Vec<_> = attempts
            .iter()
            .filter(|attempt| attempt.spell == CRYSTALLINE_ROAR)
            .collect();
        assert!(
            strikes.len() >= 3 && roars.len() >= 2,
            "native first/repeats in both Lua branches"
        );
        assert!(
            attempts.iter().all(|attempt| attempt.result.is_ok()),
            "ordinary admission only"
        );
        assert!((strikes[0].time - 7.4).abs() <= 0.1);
        assert!((roars[0].time - 12.2).abs() <= 0.1);
        let strike_finishes: Vec<_> = completions
            .iter()
            .filter(|(spell, _)| *spell == DRAGON_STRIKE)
            .map(|(_, time)| *time)
            .collect();
        // BossAI waits for any admitted boss cast still in progress.
        let cast_end_after = |due: f32, spell: u32| {
            attempts
                .iter()
                .filter(|other| other.spell != spell)
                .map(|other| (other.time, other.time + native_cast_secs(&h, other.spell)))
                .find(|&(start, end)| start <= due && end > due)
                .map_or(due, |(_, end)| end)
        };
        for (index, attempt) in strikes.iter().enumerate().skip(1) {
            let due = strike_finishes[index - 1] + strike_repeat;
            let expected = cast_end_after(due, DRAGON_STRIKE);
            assert!(
                (attempt.time - expected).abs() <= 0.1,
                "Strike after actual success+Lua delay/native cast wait: {attempt:?} expected{expected}"
            );
        }
        for pair in roars.windows(2) {
            let due = pair[0].time + roar_repeat;
            let expected = cast_end_after(due, CRYSTALLINE_ROAR);
            assert!(
                (pair[1].time - expected).abs() <= 0.1,
                "Roar Lua countdown/native cast wait: {pair:?} expected{expected}"
            );
        }
        for spell in [DRAGON_STRIKE, CRYSTALLINE_ROAR] {
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|event| event.kind == CombatLogKind::Damage
                        && event.amount > 0
                        && event.source == Some(boss.to_bits())
                        && event.spell_id == Some(spell)),
                "native scheduled damage{spell}"
            );
        }
        let destruction = if difficulty == 23 { 388804 } else { 385399 };
        assert_eq!(cast_count(&h, boss, destruction), 1, "first sourced alert");
        let eruption = attempts
            .iter()
            .find(|attempt| attempt.spell == ARCANE_ERUPTION);
        assert!(
            eruption.is_some_and(|attempt| attempt.time >= 28.2 - 0.1),
            "Lua 28.2s Arcane Eruption request"
        );
        assert_eq!(
            cast_count(&h, boss, BRITTLE),
            0,
            "no Brittle without a health threshold"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|start| start.encounter_id == 2584 && start.difficulty_id == difficulty)
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
            instance_script::get_boss_state(h.app.world_mut(), map, ORDINAL),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << 2,
            "actual2584 completionbit2, savedordinal3 independent"
        );
        assert_eq!(
            h.inbox(0)
                .ends
                .iter()
                .filter(|end| end.encounter_id == 2584 && end.success)
                .count(),
            1
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = cast_count(&h, boss, DRAGON_STRIKE) + cast_count(&h, boss, CRYSTALLINE_ROAR);
        h.advance(30.0);
        assert_eq!(
            cast_count(&h, boss, DRAGON_STRIKE) + cast_count(&h, boss, CRYSTALLINE_ROAR),
            before,
            "corpse silence"
        );
    }
}

#[test]
fn encounter_coverage_umbrelskul_cancelled_strike_has_no_success_repeat() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .insert_resource(super::cast_admission::CastAdmissions {
            actor: boss,
            origin,
            entries: Vec::new(),
        });
    h.engage(boss);
    h.advance(7.5);
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(boss)
            .unwrap()
            .spell_id,
        DRAGON_STRIKE
    );
    crate::spell_cast::cancel_cast(h.app.world_mut(), boss);
    h.advance(20.0);
    assert_eq!(
        cast_count(&h, boss, DRAGON_STRIKE),
        0,
        "cancelled native cast never completes"
    );
    assert_eq!(
        h.app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries
            .iter()
            .filter(|attempt| attempt.spell == DRAGON_STRIKE)
            .count(),
        1,
        "Lua repeats only from actual success"
    );
    assert_eq!(
        cast_count(&h, boss, CRYSTALLINE_ROAR),
        1,
        "independent real native Roar remains active"
    );
}

#[test]
fn encounter_coverage_umbrelskul_evade_rearms_without_credit() {
    let (mut h, boss, map) = umbrelskul_copy(205);
    h.engage(boss);
    h.advance(11.0);
    assert_eq!(cast_count(&h, boss, DRAGON_STRIKE), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
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
    h.engage(boss);
    h.advance(9.6);
    assert_eq!(
        cast_count(&h, boss, DRAGON_STRIKE),
        1,
        "source7.4+native2.5 earliest completion"
    );
    h.advance(0.6);
    assert_eq!(
        cast_count(&h, boss, DRAGON_STRIKE),
        2,
        "repull re-arms native first request"
    );
}

const ARCANE_ERUPTION: u32 = 385075;
const UNLEASHED_DESTRUCTION: u32 = 385399;
const BRITTLE: u32 = 386746;
const DETONATING_CRYSTAL: u32 = 195138;

/// Untargeted NPC casts reach no player's combat log; read server-observed events.
fn cast_starts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastStart
                && event.source == Some(boss.to_bits())
                && event.spell_id == Some(spell)
        })
        .count()
}

#[test]
fn umbrelskul_arcane_eruption_follows_lua_energy_clock_and_native_cast() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    h.engage(boss);
    h.advance(28.1);
    assert_eq!(cast_starts(&h, boss, ARCANE_ERUPTION), 0);
    // The request waits for any cast in progress (BossAI casting gate).
    h.advance(3.0);
    assert_eq!(
        cast_starts(&h, boss, ARCANE_ERUPTION),
        1,
        "Lua 28.2s first Arcane Eruption"
    );
}

#[test]
fn umbrelskul_unleashed_destruction_repeats_on_lua_normal_clock() {
    let (mut h, boss, _) = umbrelskul_copy(205);
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    h.engage(boss);
    h.advance(105.0);
    assert_eq!(cast_starts(&h, boss, UNLEASHED_DESTRUCTION), 1);
    h.advance(15.0);
    assert_eq!(
        cast_starts(&h, boss, UNLEASHED_DESTRUCTION),
        2,
        "Lua 54.7s repeat from the first cast start"
    );
}

#[test]
fn umbrelskul_brittle_interrupts_cast_once_per_lua_threshold_without_crystals() {
    let (mut h, boss, map) = umbrelskul_copy(205);
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let brittles = |h: &Harness| {
        h.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(boss.to_bits())
                    && event.spell_id == Some(BRITTLE)
            })
            .count()
    };
    h.engage(boss);
    h.advance(0.05);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, UNLEASHED_DESTRUCTION, None, false)
        .expect("native385399 admission");
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(boss)
            .is_some()
    );
    h.hit_pct(boss, 30.0);
    h.advance(0.1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(boss)
            .is_none_or(|cast| cast.spell_id != UNLEASHED_DESTRUCTION),
        "Brittle interrupts Unleashed Destruction"
    );
    assert_eq!(brittles(&h), 1, "100% to 70% crosses 75%");
    h.advance(4.0);
    assert_eq!(
        cast_count(&h, boss, UNLEASHED_DESTRUCTION),
        0,
        "Lua: the interrupted Destruction is not recast"
    );
    h.hit_pct(boss, 5.0);
    h.advance(0.1);
    assert_eq!(brittles(&h), 1, "70% to 65% crosses no threshold");
    h.hit_pct(boss, 45.0);
    h.advance(0.1);
    assert_eq!(
        brittles(&h),
        3,
        "one hit from 65% to 20% casts Brittle for 50% and 25%"
    );
    h.advance(3.0);
    let crystals = h
        .creatures_of(DETONATING_CRYSTAL)
        .into_iter()
        .filter(|&crystal| h.app.world().get::<InMap>(crystal) == Some(&map))
        .count();
    assert_eq!(
        crystals, 0,
        "UMBRELSKUL_BRITTLE_DUMMY_CHILD_UNSOURCED: no crystal from dummy base points"
    );
}
