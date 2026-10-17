//! Pinned Landslide schedules, first incoming damage crossing50%, actual summons.
use super::*;

#[test]
fn encounter_coverage_landslide_wipe_rearms_damage_trigger() {
    let (mut h, boss, map) = bound_boss(12203, 349, 4);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 3.0,
            ..origin
        },
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    h.engage(boss);
    h.advance(0.1);
    h.hit_pct(boss, 49.0);
    h.advance(0.1);
    assert!(!h.has_aura(boss, 21808), "incoming hit remains above50%");
    h.hit_pct(boss, 2.0);
    h.advance(0.1);
    assert!(h.has_aura(boss, 21808), "first below50 hit triggers");
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
    h.advance(0.2);
    assert!(!h.has_aura(boss, 21808), "wipe removes source channel");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 4),
        Some(EncounterState::NotStarted)
    );
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1_000_000.0;
    h.engage(boss);
    h.advance(0.1);
    h.hit_pct(boss, 1.0);
    h.advance(0.1);
    assert!(
        h.has_aura(boss, 21808),
        "reset re-arms incoming damage below50"
    );
}

#[test]
fn encounter_coverage_landslide_threshold_clocks_summons_and_isolated_credit() {
    let seed = super::landslide_seed_probe::find_counterexample_seed();
    eprintln!("LANDSLIDE seed={seed}");
    assert_landslide_clocks(seed);
}

fn assert_landslide_clocks(seed: u64) {
    let (mut h, boss, map) = bound_boss(12203, 349, 4);
    h.app
        .world_mut()
        .get_mut::<CreatureAiState>(boss)
        .unwrap()
        .ai = Some(super::super::super::maraudon::seeded_landslide(seed));
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 3.0,
            ..origin
        },
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut blocked = Vec::new();
    let mut summons = std::collections::HashSet::new();
    let mut channel_seen = false;
    let mut channel_start_count = None;
    let mut completed_waves = 0;
    for tick in 1..=1900 {
        if tick == 401 {
            h.hit_pct(boss, 60.0);
        }
        if tick == 421 {
            h.hit_pct(boss, 1.0);
        }
        blocked.push((
            (tick - 1) as f32 * 0.05,
            h.app.world().get::<CastState>(boss).is_some(),
        ));
        let elapsed = h.app.world().resource::<Time<Fixed>>().elapsed();
        h.advance(0.05);
        assert_eq!(
            h.app.world().resource::<Time<Fixed>>().elapsed() - elapsed,
            Duration::from_millis(50),
            "each harness frame advances exactly one simulation tick"
        );
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [18670, 5568, 21808, 21809].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
        if tick <= 400 {
            assert!(
                !h.has_aura(boss, 21808),
                "not before incoming damage crosses50%"
            );
        }
        if h.has_aura(boss, 21808) {
            let channel = h
                .app
                .world()
                .get::<CastState>(boss)
                .expect("actual source Landslide channel");
            assert_eq!(
                (channel.spell_id, channel.cast_type),
                (21808, CastType::Channel)
            );
            channel_seen = true;
            channel_start_count.get_or_insert(summons.len());
        }
        summons.extend(h.creatures_of(11783));
        if !h.has_aura(boss, 21808)
            && let Some(start_count) = channel_start_count.take()
        {
            assert_eq!(
                summons.len() - start_count,
                4,
                "8s channel with native2s period produces four actual summons"
            );
            completed_waves += 1;
        }
    }
    eprintln!(
        "LANDSLIDE source casts={casts:?}, unique summons={}",
        summons.len()
    );
    for (spell, first, repeat, cast_time, target) in [
        (18670, (5.0, 10.0), (15.0, 25.0), 1.0, player),
        (5568, (10.0, 15.0), (10.0, 20.0), 0.0, boss),
        (21808, (20.0, 20.0), (30.0, 40.0), 0.0, boss),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time - cast_time)
            .collect();
        assert!(
            super::celebras::follows_task_deadlines(&times, first, repeat, &blocked),
            "source clock{spell}: {times:?}"
        );
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, actual)| *actual == Some(target.to_bits())),
            "source target{spell}"
        );
    }
    assert!(channel_seen, "real below50 channel");
    assert!(completed_waves >= 2, "two full native21809 summon waves");
    for spell in [18670, 5568] {
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual damage{spell}"
        );
    }
    assert!(
        h.creatures_of(11783)
            .iter()
            .all(|add| *h.app.world().get::<InMap>(*add).unwrap() == map)
    );
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(
        h.creatures_of(11783).is_empty(),
        "source BossAI death cleanup"
    );
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 4),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << 4
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 426 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    let before = h.inbox(0).spell_gos.len();
    h.advance(40.0);
    assert_eq!(
        h.inbox(0).spell_gos.len(),
        before,
        "dead source actor stops all casts"
    );
}
