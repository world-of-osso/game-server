//! Pinned TC Noxxion, actual DB2 spells and TDB actors; controlled survival only.
use super::*;

fn spawned_adds(h: &mut Harness, map: InMap) -> Vec<Entity> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap)>();
    query
        .iter(world)
        .filter_map(|(entity, npc, actor_map)| {
            (npc.template_id == 13456 && *actor_map == map).then_some(entity)
        })
        .collect()
}

fn assert_spawn_phase_state(h: &Harness, boss: Entity, expected: ReactState) {
    assert_eq!(h.app.world().get::<ReactState>(boss), Some(&expected));
    let flags = h.app.world().get::<UnitFlags>(boss).unwrap().0;
    assert_eq!(
        flags & UnitFlags::NOT_SELECTABLE != 0,
        expected == ReactState::Passive
    );
}

// Intersect possible authored due-times with observed dispatch times. A channel
// can defer dispatch to its end, but cannot move the TaskContext repeat deadline.
fn follows_volley_deadlines(times: &[f32], channels: &[(f32, f32)]) -> bool {
    let millis = |time: f32| (time * 1000.0).round() as u64;
    let first = millis(times[0]);
    let (mut low, mut high) = (first.saturating_sub(50), first);
    for time in &times[1..] {
        let time = millis(*time);
        if channels
            .iter()
            .any(|(start, end)| time > millis(*start) + 50 && time + 50 < millis(*end))
        {
            return false;
        }
        let earliest = channels
            .iter()
            .find(|(_, end)| time.abs_diff(millis(*end)) <= 50)
            .map_or(time.saturating_sub(50), |(start, _)| {
                millis(*start).saturating_sub(50)
            });
        low = (low + 15_000).max(earliest);
        high = (high + 20_000).min(time);
        if low > high {
            return false;
        }
    }
    true
}

#[test]
fn encounter_coverage_noxxion_clock_observer_accepts_deferred_catchup_not_late_unblocked_casts() {
    assert!(follows_volley_deadlines(
        &[5.0, 25.0, 47.0, 62.0],
        &[(32.0, 47.0)]
    ));
    assert!(!follows_volley_deadlines(&[5.0, 26.0], &[]));
    assert!(!follows_volley_deadlines(
        &[5.0, 25.0, 40.0],
        &[(32.0, 47.0)]
    ));
}

#[test]
fn encounter_coverage_noxxion_delayed_task_repeat_preserves_due_clock() {
    let mut tasks = super::super::super::EventMap::default();
    tasks.schedule(21687, 100);
    tasks.update(150);
    assert_eq!(tasks.execute_event(), Some(21687));
    tasks.repeat_from_due_range(100, 100);
    tasks.update(49);
    assert_eq!(tasks.execute_event(), None);
    tasks.update(1);
    assert_eq!(tasks.execute_event(), Some(21687));
}

#[test]
fn encounter_coverage_noxxion_source_clocks_phase_summons_and_credit() {
    let (mut h, boss, map) = bound_boss(13282, 349, 0);
    let player = h.player();
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut adds = std::collections::HashSet::new();
    let mut phase_seen = false;
    let mut phase_ended = false;
    let mut channel_start = None;
    let mut channels = Vec::new();
    for tick in 1..=2220 {
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [21687, 22916, 21708, 21707].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
        adds.extend(spawned_adds(&mut h, map));
        let phase_active = h.has_aura(boss, 21708);
        if phase_active {
            channel_start.get_or_insert(tick as f32 * 0.05);
            let channel = h
                .app
                .world()
                .get::<CastState>(boss)
                .expect("real21708 channel");
            assert_eq!(
                (channel.spell_id, channel.cast_type),
                (21708, CastType::Channel)
            );
            phase_seen = true;
            assert_spawn_phase_state(&h, boss, ReactState::Passive);
        } else if phase_seen {
            if let Some(start) = channel_start.take() {
                channels.push((start, tick as f32 * 0.05));
            }
            phase_ended = true;
            assert_spawn_phase_state(&h, boss, ReactState::Aggressive);
        }
    }
    eprintln!(
        "NOXXION successful source casts={casts:?}, unique adds={}",
        adds.len()
    );
    let phase_times: Vec<_> = casts
        .iter()
        .filter(|(spell, _, _)| *spell == 21708)
        .map(|(_, time, _)| *time)
        .collect();
    assert!(
        phase_times.len() >= 2,
        "real source phase repeats: {phase_times:?}"
    );
    assert!(phase_times[0] >= 30.0 && phase_times[0] <= 40.1);
    assert!((50.0..=60.1).contains(&(phase_times[1] - phase_times[0])));
    assert!(
        phase_seen && phase_ended,
        "real aura apply/remove transitions"
    );
    assert_eq!(adds.len(), 10, "two real five-add waves");
    assert!(
        casts
            .iter()
            .filter(|(spell, _, _)| [21708, 21707].contains(spell))
            .all(|(_, _, target)| *target == Some(boss.to_bits())),
        "source self phase casts"
    );
    for (spell, first_min, first_max, target) in
        [(21687, 5.0, 10.0, boss), (22916, 15.0, 20.0, player)]
    {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time)
            .collect();
        assert!(times.len() >= 2, "real spell repeats {spell}");
        assert!(
            times[0] >= first_min && times[0] <= first_max + 0.1,
            "source first clock {spell}: {times:?}"
        );
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, actual)| *actual == Some(target.to_bits())),
            "source cast argument {spell}"
        );
        // Uppercut can skip a victim-less window. The actual21708 channel also
        // defers dispatch of self Volley; repeats retain the original due clock.
        if spell == 21687 {
            assert!(
                follows_volley_deadlines(&times, &channels),
                "source Volley deadlines: {times:?}, channels={channels:?}"
            );
        }
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "real victim damage {spell}"
        );
    }
    assert!(h.inbox(1).starts.is_empty());
    assert!(
        h.inbox(1)
            .spell_gos
            .iter()
            .all(|go| go.caster != boss.to_bits())
    );
    // Wait out a source phase, never clear its flag/aura to force a controlled kill.
    for _ in 0..600 {
        if !h.has_aura(boss, 21708) {
            break;
        }
        h.advance(0.05);
    }
    assert!(!h.has_aura(boss, 21708));
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
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
            .filter(|end| end.encounter_id == 422 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert!(
        spawned_adds(&mut h, map).is_empty(),
        "BossAI death despawns all its summons"
    );
    let before = spell_gos(&h, boss, 21687).len();
    h.advance(25.0);
    assert_eq!(
        spell_gos(&h, boss, 21687).len(),
        before,
        "dead boss scheduler stops"
    );
}
