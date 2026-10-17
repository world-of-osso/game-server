//! Omor source EventMap and hounds: real Normal-copy admission, not native balance.
use super::*;

#[test]
fn encounter_coverage_omor_source_clocks_shield_hounds_and_credit() {
    let (mut h, boss, map) = bound_boss(17308, 543, 0);
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 3.0,
        ..origin
    });
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut shield_seen = false;
    let mut treacherous_seen = false;
    for tick in 1..=1400 {
        if tick == 801 {
            h.hit_pct(boss, 81.0);
        }
        if tick == 901 {
            h.hit_pct(boss, 1.0);
        }
        h.advance(0.05);
        shield_seen |= h.has_aura(boss, 31901);
        treacherous_seen |= h.has_aura(player, 30695);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits()
                && [30686, 30695, 30707, 35642, 31901].contains(&go.spell_id))
            .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
    }
    eprintln!("OMOR source casts={casts:?}");
    // Go is cast completion. EventMap may defer another due event behind the
    // authored3s Shadow Bolt or2s Treacherous Aura cast; no timer is shortened.
    for (spell, first, repeat) in [
        (30686, (3.0, 3.2), (6.0, 13.2)),
        (30695, (18.0, 28.2), (30.0, 45.2)),
        (30707, (20.0, 28.2), (20.0, 31.2)),
        (35642, (25.0, 30.2), (23.0, 35.2)),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, t, _)| *t)
            .collect();
        assert!(times.len() >= 2, "actual repeat{spell}: {times:?}");
        assert!(
            times[0] >= first.0 && times[0] <= first.1,
            "first{spell}: {times:?}"
        );
        assert!(
            times
                .windows(2)
                .all(|p| p[1] - p[0] >= repeat.0 - 0.1 && p[1] - p[0] <= repeat.1),
            "dispatch-anchored repeat{spell}: {times:?}"
        );
    }
    let shields: Vec<_> = casts.iter().filter(|(id, _, _)| *id == 31901).collect();
    assert_eq!(
        shields.len(),
        1,
        "incoming threshold once despite second hit"
    );
    assert!((40.0..=43.2).contains(&shields[0].1));
    assert!(shield_seen, "actual Demonic Shield aura");
    assert!(treacherous_seen, "actual Treacherous Aura");
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.spell_id == Some(30686)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual Shadow Bolt damage"
    );
    let hounds = h.creatures_of(17540);
    assert!(hounds.len() >= 2, "actual source hound summons");
    for hound in &hounds {
        assert_eq!(h.app.world().get::<InMap>(*hound), Some(&map));
        let health = h.app.world().get::<Health>(*hound).unwrap();
        assert!(health.current > 0.0);
        assert!(
            h.app
                .world()
                .get::<shared::components::ThreatTable>(*hound)
                .is_some_and(|threat| !threat.entries().is_empty()),
            "source hounds engage"
        );
    }
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
            .filter(|end| end.encounter_id == 1891 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    h.advance(5.0);
    for hound in hounds {
        assert!(
            h.app.world().get_entity(hound).is_ok(),
            "source explicitly does not BossAi-despawn hounds"
        );
    }
}
