//! Omor source EventMap and hounds: real Normal-copy admission, not native balance.
use super::*;

fn orbital_fixture() -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss(17308, 543, 0);
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&543].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(543, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

#[test]
#[ignore = "RED: Omor Orbital target SpellScript not ported"]
fn encounter_coverage_omor_orbital_two_candidates_knocks_one_and_clears_its_threat() {
    let (mut h, boss, map) = orbital_fixture();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let players = [h.players[0].0, h.players[1].0];
    for (player, gap) in players.into_iter().zip([3.0, 8.0]) {
        h.app.world_mut().entity_mut(player).insert((
            map,
            Position {
                x: origin.x + gap,
                ..origin
            },
            shared::components::PlayerMotion::default(),
        ));
    }
    for player in players {
        let position = *h.app.world().get::<Position>(player).unwrap();
        let ground = h
            .app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .ground_in(map, Vec3::new(position.x, position.y, position.z));
        assert!(
            matches!(ground, crate::ground::GroundProbe::Supported(_)),
            "authentic Ramparts support at {position:?}: {ground:?}"
        );
    }
    h.engage(boss);
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, players[1], 0.1);
    for _ in 0..640 {
        h.advance(0.05);
        if h.has_aura(boss, 32185) {
            break;
        }
    }
    assert!(
        h.inbox(0).spell_gos.iter().any(|go| go.spell_id == 35642),
        "actual scheduled Orbital parent reached its hit boundary"
    );
    assert!(
        h.has_aura(boss, 32185),
        "source Orbital target script must cast actual Whip32185"
    );
    let falling: Vec<_> = players
        .into_iter()
        .filter(|player| {
            h.app
                .world()
                .get::<shared::components::PlayerMotion>(*player)
                .is_some_and(|motion| motion.contains(shared::components::PlayerMotion::FALLING))
        })
        .collect();
    assert_eq!(
        falling.len(),
        1,
        "source30637 knocks exactly one eligible candidate into falling"
    );
    let selected = falling[0];
    let threat = h
        .app
        .world()
        .get::<shared::components::ThreatTable>(boss)
        .unwrap();
    assert_eq!(
        threat.threat_for(selected.to_bits()),
        0.0,
        "source -100% selected threat"
    );
    let other = players
        .into_iter()
        .find(|player| *player != selected)
        .unwrap();
    assert!(
        threat.threat_for(other.to_bits()) > 0.0,
        "unselected threat retained"
    );
}

#[test]
#[ignore = "RED: Omor Whip falling-player periodic AuraScript not ported"]
fn encounter_coverage_omor_orbital_whip_targets_falling_threat_not_its_caster() {
    let (mut h, boss, map) = orbital_fixture();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let players = [h.players[0].0, h.players[1].0];
    for (player, gap) in players.into_iter().zip([3.0, 8.0]) {
        h.app.world_mut().entity_mut(player).insert((
            map,
            Position {
                x: origin.x + gap,
                ..origin
            },
            shared::components::PlayerMotion::default(),
        ));
        crate::creature_aggro::add_threat(h.app.world_mut(), boss, player, 1.0);
    }
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30637, Some(players[0]), true)
        .expect("authentic source knock");
    assert!(
        h.app
            .world()
            .get::<shared::components::PlayerMotion>(players[0])
            .unwrap()
            .contains(shared::components::PlayerMotion::FALLING),
        "actual knock starts falling"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 32185, Some(boss), true)
        .expect("authentic source Whip");
    h.advance(1.9);
    assert!(
        !h.inbox(0).spell_gos.iter().any(|go| go.spell_id == 30638),
        "no early periodic child"
    );
    h.advance(0.15);
    let children: Vec<_> = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.spell_id == 30638)
        .map(|go| go.target)
        .collect();
    assert_eq!(
        children,
        vec![Some(players[0].to_bits())],
        "only falling player threat, never self/grounded player"
    );
}

#[test]
fn encounter_coverage_omor_orbital_singleton_keeps_hit_without_script_effects() {
    for gap in [3.0, 25.0] {
        let (mut h, boss, _) = orbital_fixture();
        let player = h.player();
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + gap,
            ..origin
        });
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 35642, Some(boss), true)
            .expect("actual singleton Orbital");
        h.advance(0.05);
        let parent = h
            .inbox(0)
            .spell_gos
            .iter()
            .find(|go| go.spell_id == 35642)
            .expect("singleton parent publication");
        assert_eq!(
            parent.hit_targets,
            vec![player.to_bits()],
            "retain source singleton even outside20yd"
        );
        assert!(
            !h.has_aura(boss, 32185),
            "singleton cannot continue into Whip"
        );
        assert!(
            !h.inbox(0).spell_gos.iter().any(|go| go.spell_id == 30637),
            "singleton no knock"
        );
    }
}

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
