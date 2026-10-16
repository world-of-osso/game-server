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
    assert!(h.has_aura(boss, 22857), "actual Retaliation aura");
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
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
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
    h.engage(boss);
    h.advance(0.1);
    h.hit_pct(boss, 81.0);
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
