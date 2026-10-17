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
    h.advance(2.05);
    let selected_inbox = players
        .iter()
        .position(|player| *player == selected)
        .unwrap();
    assert!(
        h.inbox(selected_inbox)
            .log
            .iter()
            .any(|event| event.spell_id == Some(30638)
                && event.target == Some(selected.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "Whip still targets the genuine falling player after its threat becomes zero"
    );
}

#[test]
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
        !h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(30638)),
        "no early periodic child damage"
    );
    let falling_health = h.health(players[0]).current;
    let grounded_health = h.health(players[1]).current;
    let caster_health = h.health(boss).current;
    h.advance(0.15);
    // cast_triggered deliberately has no SpellGo/CastSuccess publication.
    let children: Vec<_> = h
        .inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(30638) && event.kind == CombatLogKind::Damage && event.amount > 0
        })
        .map(|event| event.target)
        .collect();
    assert_eq!(
        children,
        vec![Some(players[0].to_bits())],
        "only falling player threat, never self/grounded player"
    );
    assert!(
        h.health(players[0]).current < falling_health,
        "actual authored child damage"
    );
    assert_eq!(
        h.health(players[1]).current,
        grounded_health,
        "grounded player unaffected"
    );
    assert_eq!(
        h.health(boss).current,
        caster_health,
        "prevent default self-trigger damage"
    );
}

#[test]
fn encounter_coverage_omor_whip_pull_reverses_real_airborne_motion() {
    let (mut h, boss, map) = orbital_fixture();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert((
        map,
        Position {
            x: origin.x + 3.0,
            ..origin
        },
        shared::components::PlayerMotion::default(),
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), boss, player, 1.0);
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30637, Some(player), true)
        .expect("actual source knock30637");
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 32185, Some(boss), true)
        .expect("actual source periodic Whip32185");
    h.advance(1.9);
    let before = *h.app.world().get::<Position>(player).unwrap();
    h.advance(0.15);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.spell_id == Some(30638)
                && event.target == Some(player.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "actual authored periodic30638 child reached target"
    );
    let pulled = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        pulled.x > before.x,
        "genuine outgoing knock traveled before pull"
    );
    assert!(
        h.app
            .world()
            .get::<shared::components::PlayerMotion>(player)
            .unwrap()
            .contains(shared::components::PlayerMotion::FALLING),
        "actual airborne target"
    );
    h.advance(0.1);
    let after = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        after.x < pulled.x,
        "effect124 must reverse outbound motion toward caster: {pulled:?} -> {after:?}"
    );
    assert!(
        ((pulled.x - after.x) - 1.31).abs() < 0.02,
        "authored MiscValue131 gives13.1yd/s, not teleport or outgoing15yd/s: {pulled:?} -> {after:?}"
    );
}

#[test]
fn encounter_coverage_omor_whip_pull_uses_source_3d_ballistics_and_reach() {
    let (mut h, boss, map) = orbital_fixture();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert((
        map,
        Position {
            x: origin.x + 3.0,
            ..origin
        },
        shared::components::PlayerMotion::default(),
    ));
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30637, Some(player), true)
        .expect("actual30637 launch");
    h.advance(1.9);
    let start = *h.app.world().get::<Position>(player).unwrap();
    // TDB1210 creature_model_info18237 CombatReach3, source Omor scale1.
    let contact = Vec3::new(origin.x + 3.0, origin.y, origin.z);
    let crate::ground::GroundProbe::Supported(height) = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, contact)
    else {
        panic!("authentic combat-reach destination must have ground")
    };
    let contact = Vec3::new(contact.x, height, contact.z);
    let from = Vec3::new(start.x, start.y, start.z);
    let distance = from.distance(contact);
    let height_delta = contact.y - start.y;
    let speed = 13.1;
    let gravity = shared::skyriding::GRAVITY;
    let vertical_speed = (2.0 * speed * speed * height_delta + gravity * distance * distance)
        / (2.0 * speed * distance);
    let expected_y = start.y + vertical_speed * 0.1 - 0.5 * gravity * 0.01;
    let distance_2d = start.x - contact.x;
    let incorrect_speed = (2.0 * speed * speed * height_delta
        + gravity * distance_2d * distance_2d)
        / (2.0 * speed * distance_2d);
    assert!(
        (vertical_speed - incorrect_speed).abs() * 0.1 > 0.1,
        "raised real knock discriminates Blizzard3D formula from2D"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), boss, 30638, Some(player), true)
        .expect("actual child30638");
    assert_eq!(
        *h.app.world().get::<Position>(player).unwrap(),
        start,
        "launch is an impulse, not teleportation"
    );
    h.advance(0.1);
    let after = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        (after.x - (start.x - 1.31)).abs() < 0.02,
        "pull overrides outgoing direction at authored13.1: {start:?} -> {after:?}"
    );
    assert!(
        (after.y - expected_y).abs() < 0.02,
        "source3D distance/combat-reach floor/signed vertical speed: expected{expected_y}, actual{after:?}"
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
        let health = h.health(player).current;
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 35642, Some(boss), false)
            .expect("actual ordinary singleton Orbital");
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
        assert_eq!(
            h.health(player).current,
            health,
            "singleton no knock damage"
        );
    }
}

#[test]
fn encounter_coverage_omor_orbital_exact_two_dimensional_cutoff() {
    // Source exact2D<=20 includes the boundary/raised target, not 20.01yd.
    for (first_x, first_height, selected) in
        [(20.0, 0.0, true), (20.01, 0.0, false), (19.0, 25.0, true)]
    {
        let (mut h, boss, map) = orbital_fixture();
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        let players = [h.players[0].0, h.players[1].0];
        for (player, x, height) in [(players[0], first_x, first_height), (players[1], 25.0, 0.0)] {
            h.app.world_mut().entity_mut(player).insert((
                map,
                Position {
                    x: origin.x + x,
                    y: origin.y + height,
                    ..origin
                },
            ));
        }
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 35642, Some(boss), false)
            .expect("ordinary Orbital cutoff cast");
        h.advance(0.05);
        let parent = h
            .inbox(0)
            .spell_gos
            .iter()
            .find(|go| go.spell_id == 35642)
            .expect("actual Orbital cutoff receipt");
        let expected = if selected {
            vec![players[0].to_bits()]
        } else {
            Vec::new()
        };
        assert_eq!(
            parent.hit_targets, expected,
            "cutoff x={first_x} height={first_height}"
        );
        assert_eq!(
            h.has_aura(boss, 32185),
            selected,
            "no hook after filtering all candidates"
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
