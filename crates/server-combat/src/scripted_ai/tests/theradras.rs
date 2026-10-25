//! Pinned Princess schedules, real damage/fear/channel and fixed death spirit.
//! Thrash3391 dispatch is not proof of its unsupported extra-attacks effect19.
use super::*;

#[test]
fn encounter_coverage_theradras_source_clocks_effects_spirit_and_credit() {
    let (mut h, boss, map) = bound_boss(12201, 349, 7);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            // DB2 Monster Shoot range54: Boulder requires at least5yd.
            x: origin.x + 8.0,
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
    let mut fear_seen = false;
    let mut dust_seen = false;
    for tick in 1..=1700 {
        // Controlled kiting fixture: real melee pursuit otherwise closes to2yd
        // and correctly rejects every Boulder (DB2 min5yd). Not movement proof.
        let boss_position = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: boss_position.x + 8.0,
            ..boss_position
        });
        blocked.push((
            (tick - 1) as f32 * 0.05,
            h.app.world().get::<CastState>(boss).is_some(),
        ));
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [21909, 21832, 3391, 21869].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
        fear_seen |= h.has_aura(player, 21869);
        dust_seen |= h.has_aura(boss, 21909);
    }
    eprintln!("THERADRAS source casts={casts:?}, dust={dust_seen}, fear={fear_seen}");
    for (spell, first, repeat, cast_time, target) in [
        (21909, (20.0, 25.0), (20.0, 25.0), 0.0, boss),
        (21832, (20.0, 30.0), (15.0, 25.0), 2.0, player),
        (3391, (10.0, 20.0), (10.0, 20.0), 0.0, boss),
        (21869, (25.0, 35.0), (30.0, 40.0), 0.0, boss),
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
    assert!(dust_seen, "real Dust Field aura");
    assert!(fear_seen, "real nearby Repulsive Gaze fear");
    for spell in [21832, 21868] {
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "actual spell damage{spell}"
        );
    }
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    let spirit = h.creatures_of(12238);
    assert_eq!(spirit.len(), 1, "actual source death spirit");
    let spirit = spirit[0];
    assert_eq!(*h.app.world().get::<InMap>(spirit).unwrap(), map);
    let position = h.app.world().get::<Position>(spirit).unwrap();
    assert!(
        (position.x - 28.1887).abs() < 0.001
            && (position.y + 123.161).abs() < 0.001
            && (position.z + 62.3964).abs() < 0.001,
        "authored fixed spirit WoW coordinates"
    );
    assert!(
        (h.app.world().get::<Rotation>(spirit).unwrap().y
            - (4.31096 + std::f32::consts::FRAC_PI_2))
            .abs()
            < 0.001,
        "authored spirit orientation"
    );
    let template = h
        .app
        .world()
        .resource::<WorldData>()
        .template_for(12238, 1)
        .unwrap();
    assert_eq!(
        template.health_modifier, 2.0,
        "source spirit health, not legacy1.35"
    );
    assert_eq!(
        (template.min_level, template.max_level),
        (10, 30),
        "actual Normal1CT281, not legacy60"
    );
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, 7),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << 7
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 429 && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert!(
        h.inbox(1)
            .spell_gos
            .iter()
            .all(|go| go.caster != boss.to_bits())
    );
    let before = spell_gos(&h, boss, 21909).len()
        + spell_gos(&h, boss, 21832).len()
        + spell_gos(&h, boss, 3391).len()
        + spell_gos(&h, boss, 21869).len();
    h.advance(40.0);
    assert_eq!(
        h.creatures_of(12238),
        vec![spirit],
        "manual spirit survives corpse cleanup"
    );
    assert_eq!(
        spell_gos(&h, boss, 21909).len()
            + spell_gos(&h, boss, 21832).len()
            + spell_gos(&h, boss, 3391).len()
            + spell_gos(&h, boss, 21869).len(),
        before,
        "dead source caster silent"
    );
}
