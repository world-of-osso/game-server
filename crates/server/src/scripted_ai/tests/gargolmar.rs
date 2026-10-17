//! Pinned Gargolmar EventMap, incoming-damage phases and isolated credit.
use super::*;

fn pull_gargolmar() -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss(17306, 543, 2);
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
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
    (h, boss, map)
}

#[test]
fn encounter_coverage_gargolmar_source_clocks_thresholds_and_credit() {
    let (mut h, boss, map) = pull_gargolmar();
    let player = h.player();
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut retaliation_seen = false;
    for tick in 1..=1400 {
        if tick == 401 {
            h.hit_pct(boss, 61.0);
        }
        if tick == 601 {
            h.hit_pct(boss, 21.0);
        }
        if tick == 701 {
            h.hit_pct(boss, 1.0);
        }
        h.advance(0.05);
        retaliation_seen |= h.has_aura(boss, 22857);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [30641, 34645, 30642, 22857].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
    }
    eprintln!("GARGOLMAR source casts={casts:?}");
    for (spell, first, repeat) in [
        (30641, (5.0, 10.0), (5.0, 15.0)),
        (34645, (3.0, 5.0), (15.0, 40.0)),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time)
            .collect();
        assert!(times.len() >= 2, "real source casts{spell}: {times:?}");
        assert!(
            times[0] >= first.0 && times[0] <= first.1 + 0.2,
            "first{spell}: {times:?}"
        );
        assert!(
            times
                .windows(2)
                .all(|pair| pair[1] - pair[0] >= repeat.0 - 0.1
                    && pair[1] - pair[0] <= repeat.1 + 0.2),
            "repeats{spell}: {times:?}"
        );
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, target)| *target == Some(player.to_bits())),
            "victim/single-threat farthest target{spell}"
        );
    }
    let heals: Vec<_> = casts.iter().filter(|(id, _, _)| *id == 30642).collect();
    assert_eq!(
        heals.len(),
        1,
        "request heal once despite later incoming hits"
    );
    assert!((heals[0].1 - 20.0).abs() <= 0.2 && heals[0].2 == Some(boss.to_bits()));
    let retaliation: Vec<_> = casts.iter().filter(|(id, _, _)| *id == 22857).collect();
    assert_eq!(retaliation.len(), 2, "one threshold task,30s repeat");
    assert!((retaliation[0].1 - 30.0).abs() <= 0.2);
    assert!((retaliation[1].1 - retaliation[0].1 - 30.0).abs() <= 0.2);
    assert!(
        retaliation
            .iter()
            .all(|(_, _, target)| *target == Some(boss.to_bits()))
    );
    for spell in [30641, 34645] {
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
        h.has_aura(player, 30641),
        "actual Mortal Wound heal reduction"
    );
    assert!(
        retaliation_seen,
        "actual Retaliation aura during its sourced 8s duration"
    );
    assert!(!h.has_aura(boss, 22857), "last Retaliation expires at 68s");
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << 2
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 1893 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    let before = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits())
        .count();
    h.advance(40.0);
    assert_eq!(
        h.inbox(0)
            .spell_gos
            .iter()
            .filter(|go| go.caster == boss.to_bits())
            .count(),
        before,
        "postdeath silence"
    );
}

#[test]
fn encounter_coverage_gargolmar_real_evade_rearms_damage_thresholds() {
    let (mut h, boss, map) = pull_gargolmar();
    h.advance(0.1);
    h.hit_pct(boss, 81.0);
    h.advance(0.2);
    assert_eq!(spell_gos(&h, boss, 30642).len(), 1);
    assert_eq!(spell_gos(&h, boss, 22857).len(), 1);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 500.0,
        ..origin
    });
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    for _ in 0..200 {
        h.advance(0.05);
        if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
            != Some(&crate::creature_ai::CreatureState::Returning)
        {
            break;
        }
    }
    assert_ne!(
        h.app.world().get::<crate::creature_ai::CreatureState>(boss),
        Some(&crate::creature_ai::CreatureState::Returning),
        "complete actual home return before re-pull"
    );
    let health = h.app.world().get::<Health>(boss).unwrap();
    assert!(
        health.current > 0.0,
        "real home return leaves a living boss"
    );
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 2),
        Some(EncounterState::NotStarted)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 1893 && !end.success)
    );
    let home = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: home.x + 3.0,
        ..home
    });
    h.engage(boss);
    h.advance(0.1);
    // Fixture omits regeneration; boss remains below both thresholds.
    h.hit_pct(boss, 1.0);
    h.advance(0.2);
    assert_eq!(
        spell_gos(&h, boss, 30642).len(),
        2,
        "real re-pull request rearm"
    );
    assert_eq!(
        spell_gos(&h, boss, 22857).len(),
        2,
        "real re-pull retaliation rearm"
    );
}

#[test]
fn encounter_coverage_gargolmar_surge_selects_farthest_living_threat() {
    let (mut h, boss, map) = pull_gargolmar();
    let far = h.players[1].0;
    assert_ne!(
        far,
        h.player(),
        "farthest threat is not the high-threat melee victim"
    );
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let destination = Position {
        x: origin.x + 20.0,
        ..origin
    };
    h.app.world_mut().entity_mut(far).insert((map, destination));
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, far, 0.1);
    for _ in 0..120 {
        h.advance(0.05);
        if !spell_gos(&h, boss, 34645).is_empty() {
            break;
        }
    }
    assert_eq!(
        spell_gos(&h, boss, 34645).last().unwrap().target,
        Some(far.to_bits())
    );
    let at_launch = *h.app.world().get::<Position>(boss).unwrap();
    let distance = |p: Position| {
        Vec3::new(p.x, p.y, p.z).distance(Vec3::new(destination.x, destination.y, destination.z))
    };
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), far).unwrap();
    assert!(
        distance(at_launch) > reach + 1.0,
        "charge must travel, not teleport at launch"
    );
    let speed = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(34645)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .speed;
    let expected_frames =
        ((1.0 + (distance(at_launch) - reach) * 1000.0 / speed) / 50.0).ceil() as usize;
    let mut previous = at_launch;
    let mut impact = None;
    for frame in 1..=20 {
        h.advance(0.05);
        let position = *h.app.world().get::<Position>(boss).unwrap();
        let traveled = Vec3::new(
            position.x - previous.x,
            position.y - previous.y,
            position.z - previous.z,
        )
        .length();
        assert!(
            traveled <= speed * 0.05 + 0.001,
            "charge exceeds authored velocity: {traveled}"
        );
        previous = position;
        if h.inbox(1).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(far.to_bits())
                && event.spell_id == Some(34645)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert_eq!(
                frame, expected_frames,
                "source spline duration owns impact time"
            );
            impact = Some(position);
            break;
        }
    }
    let at_impact = impact.expect("source Surge must damage selected distant threat after travel");
    eprintln!(
        "SURGE launch={at_launch:?}, impact={at_impact:?}, target={destination:?}, reach={reach}"
    );
    assert!(
        distance(at_impact) <= reach + 0.1,
        "source charge arrives before distant impact: remaining={}",
        distance(at_impact)
    );
}

#[test]
fn encounter_coverage_gargolmar_request_hits_real_watcher_and_heals() {
    let (mut h, boss, map) = pull_gargolmar();
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (watcher, _) = spawn_authored_actor(&mut h.app, &conn, 17309, map, 1);
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(watcher).insert(Position {
        x: origin.x + 4.0,
        ..origin
    });
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(watcher).insert(visibility);
    h.advance(0.1);
    h.hit_pct(boss, 61.0);
    let before = h.app.world().get::<Health>(boss).unwrap().current;
    h.advance(5.0);
    assert_eq!(
        spell_gos(&h, boss, 30642).len(),
        1,
        "actual incoming threshold request"
    );
    assert!(
        spell_gos(&h, watcher, 30643)
            .iter()
            .any(|go| go.target == Some(boss.to_bits())),
        "real SmartAI SpellHit30642 selects action invoker for Heal30643"
    );
    // log_event sends only to source/target owners, not nearby observers of
    // two unowned NPCs. Prove the effect through actual health, not that inbox.
    let healed = h.app.world().get::<Health>(boss).unwrap().current - before;
    assert!(
        (29.0..=32.0).contains(&healed),
        "actual source30±5% heal: {healed}"
    );
    for other_heal in [8362, 12039] {
        assert!(
            spell_gos(&h, watcher, other_heal).is_empty(),
            "no unrelated healer cast{other_heal}"
        );
    }
}
