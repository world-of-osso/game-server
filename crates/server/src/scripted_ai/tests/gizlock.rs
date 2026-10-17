//! Source SAI with actual ranged admission and channel deferral; survival is staged.
use super::*;

pub(super) fn assert_source_clock(
    times: &[f32],
    first: (f32, f32),
    repeat: (f32, f32),
    blocked: &[(f32, bool)],
) {
    assert!(times.len() >= 2, "source repeats: {times:?}");
    const DISPATCH_GRACE_MS: i64 = 150;
    const SAMPLE_GUARD_MS: i64 = 100;
    let millis = |seconds: f32| (seconds * 1000.0).round() as i64;
    let mut previous = 0.0;
    for (index, time) in times.iter().copied().enumerate() {
        let (min, max) = if index == 0 { first } else { repeat };
        assert!(
            time >= previous + min - 0.01,
            "early source dispatch: {times:?}"
        );
        let dispatch_ms = millis(time);
        let latest_due_ms = millis(previous + max);
        if dispatch_ms > latest_due_ms + DISPATCH_GRACE_MS {
            // Include exact protected endpoints: an open50ms window can discard
            // every actual50ms sample. Integer ms also eliminate f32 endpoint drift.
            let overdue: Vec<_> = blocked
                .iter()
                .filter(|(tick, _)| {
                    let tick_ms = millis(*tick);
                    tick_ms >= latest_due_ms + SAMPLE_GUARD_MS
                        && tick_ms <= dispatch_ms - SAMPLE_GUARD_MS
                })
                .collect();
            assert!(
                !overdue.is_empty() && overdue.iter().all(|(_, casting)| *casting),
                "late unblocked dispatch: {times:?}, overdue={overdue:?}"
            );
        }
        previous = time;
    }
}

#[test]
fn encounter_coverage_gizlock_clock_observer_accepts_frame_boundary_blocked_delay() {
    // Real Vyletongue gate trace: Flash ends11.2s; the7.05s Shoot's latest
    // deadline is10.95s. The old strict (11.05,11.1) window drops both samples.
    let blocked = [(11.05, true), (11.1, true)];
    assert_source_clock(&[0.05, 3.65, 7.05, 11.2], (0.0, 0.0), (2.3, 3.9), &blocked);
}

#[test]
#[should_panic(expected = "late unblocked dispatch")]
fn encounter_coverage_gizlock_clock_observer_rejects_frame_boundary_unblocked_delay() {
    let blocked = [(11.05, true), (11.1, false)];
    assert_source_clock(&[0.05, 3.65, 7.05, 11.2], (0.0, 0.0), (2.3, 3.9), &blocked);
}

#[test]
fn encounter_coverage_gizlock_clock_observer_accepts_only_blocked_delay() {
    let blocked: Vec<_> = (0..=180).map(|tick| (tick as f32 * 0.05, true)).collect();
    assert_source_clock(&[0.05, 9.0], (0.0, 0.0), (2.3, 3.9), &blocked);
}

#[test]
#[should_panic(expected = "late unblocked dispatch")]
fn encounter_coverage_gizlock_clock_observer_rejects_unblocked_delay() {
    let blocked: Vec<_> = (0..=180).map(|tick| (tick as f32 * 0.05, false)).collect();
    assert_source_clock(&[0.05, 9.0], (0.0, 0.0), (2.3, 3.9), &blocked);
}

#[test]
#[should_panic(expected = "early source dispatch")]
fn encounter_coverage_gizlock_clock_observer_rejects_early_repeat() {
    assert_source_clock(&[0.05, 1.0], (0.0, 0.0), (2.3, 3.9), &[]);
}

#[test]
fn encounter_coverage_gizlock_source_clocks_effects_talk_and_isolated_credit() {
    const BIT: u32 = 5;
    const ENCOUNTER: u32 = 427;
    let (mut h, boss, map) = bound_boss(13601, 349, BIT);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let yaw = h.app.world().get::<Rotation>(boss).unwrap().y;
    // Genuine 5yd ranged minimum, 10yd Dragon Gun cone; do not alter flags/range.
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 8.0 * yaw.sin(),
            z: origin.z + 8.0 * yaw.cos(),
            ..origin
        },
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    // Headless fixture has no interest plugin. Supply the caster's recipient set.
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut blocked = Vec::new();
    let mut gun_seen = false;
    for tick in 1..=1500 {
        blocked.push((
            (tick - 1) as f32 * 0.05,
            h.app.world().get::<CastState>(boss).is_some(),
        ));
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [16100, 9143, 21833, 29419].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
        if h.has_aura(boss, 21833) {
            gun_seen = true;
            let channel = h
                .app
                .world()
                .get::<CastState>(boss)
                .expect("actual Dragon Gun channel");
            assert_eq!(
                (channel.spell_id, channel.cast_type),
                (21833, CastType::Channel)
            );
        }
        assert!(
            !h.has_aura(player, 29419),
            "Flash Bomb admits beasts, not the human player"
        );
    }
    eprintln!("GIZLOCK source casts={casts:?}");
    for (spell, first, repeat, cast_time, target) in [
        (16100, (0.0, 0.0), (2.3, 3.9), 0.0, player),
        (9143, (4.0, 4.0), (12.0, 12.0), 2.0, player),
        (21833, (6.0, 9.0), (16.0, 19.0), 0.0, player),
        (29419, (25.0, 25.0), (25.0, 25.0), 0.0, boss),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time - cast_time)
            .collect();
        assert_source_clock(&times, first, repeat, &blocked);
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, actual)| *actual == Some(target.to_bits())),
            "authored cast argument {spell}"
        );
    }
    assert!(gun_seen, "actual self periodic-trigger aura");
    for spell in [16100, 9143, 21910] {
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual damage {spell}"
        );
    }
    let line = "Mine! Mine! Mine!  Gizlock is the ruler of this domain!  You shall never reveal my presence!";
    assert_eq!(
        h.inbox(0)
            .chat
            .iter()
            .filter(|message| message.content == line
                && message.channel == ChatType::MonsterYell(boss.to_bits()))
            .count(),
        1,
        "actual aggro Talk0"
    );
    assert!(h.inbox(1).chat.is_empty());
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
            .any(|start| start.encounter_id == ENCOUNTER)
    );
    assert!(h.inbox(1).starts.is_empty());
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
    let before = h.inbox(0).spell_gos.len();
    h.advance(30.0);
    assert_eq!(h.inbox(0).spell_gos.len(), before, "no postdeath spells");
}
