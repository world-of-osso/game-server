//! Installed Lord Vyletongue SAI; source range event, spells and isolated credit.
use super::*;

#[test]
fn encounter_coverage_vyletongue_source_range_casts_and_isolated_credit() {
    const BIT: u32 = 2;
    const ENCOUNTER: u32 = 424;
    let (mut h, boss, map) = bound_boss(12236, 349, BIT);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    let yaw = h.app.world().get::<Rotation>(boss).unwrap().y;
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 10.0 * yaw.sin(),
            z: origin.z + 10.0 * yaw.cos(),
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
    let mut stun_seen = false;
    let mut blink_displacements = Vec::new();
    for tick in 1..=1300 {
        blocked.push((
            (tick - 1) as f32 * 0.05,
            h.app.world().get::<CastState>(boss).is_some(),
        ));
        let before = *h.app.world().get::<Position>(boss).unwrap();
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        for go in &gos[cursor..] {
            if go.caster != boss.to_bits() {
                continue;
            }
            if [16100, 21390, 21655, 7964].contains(&go.spell_id) {
                casts.push((go.spell_id, tick as f32 * 0.05, go.target));
            }
            if go.spell_id == 21655 {
                let after = h.app.world().get::<Position>(boss).unwrap();
                let displacement =
                    Vec3::new(after.x - before.x, after.y - before.y, after.z - before.z).length();
                blink_displacements.push(displacement);
            }
        }
        cursor = gos.len();
        stun_seen |= h.has_aura(player, 7964);
        if tick < 400 {
            assert!(
                !h.has_aura(player, 7964),
                "10yd victim outside Flash before first Blink"
            );
        }
    }
    eprintln!(
        "VYLETONGUE source casts={casts:?}, stun={stun_seen}, Blink displacement={blink_displacements:?}"
    );
    let range_times: Vec<_> = casts
        .iter()
        .filter(|(id, time, _)| *id == 21390 && *time < 19.5)
        .map(|(_, time, _)| *time)
        .collect();
    assert!(
        range_times.len() >= 2,
        "source EVENT_RANGE21390 first and repeat: {range_times:?}"
    );
    assert!(range_times[0] <= 0.1, "range event initially ready");
    super::gizlock::assert_source_clock(&range_times, (0.0, 0.0), (7.0, 9.5), &blocked);
    for (spell, first, repeat, cast_time) in [
        (16100, (0.0, 0.0), (2.3, 3.9), 0.0),
        (7964, (8.0, 12.0), (14.0, 18.0), 1.5),
        (21655, (20.0, 30.0), (20.0, 30.0), 0.0),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, time, _)| *id == spell && (spell != 16100 || *time < 19.5))
            .map(|(_, time, _)| *time - cast_time)
            .collect();
        super::gizlock::assert_source_clock(&times, first, repeat, &blocked);
    }

    assert!(blink_displacements.len() >= 2, "real21655 cast repeats");
    assert!(
        blink_displacements
            .iter()
            .all(|distance| (*distance - 20.0).abs() < 0.01),
        "actual source20yd caster-front Blink: {blink_displacements:?}"
    );
    for spell in [16100, 21390] {
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
    assert_eq!(
        h.inbox(0).spell_gos.len(),
        before,
        "dead actor stops source casts"
    );
}

#[test]
fn encounter_coverage_vyletongue_range_boundaries_recheck_after_source_half_second() {
    for vertical in [false, true] {
        let (mut h, boss, _) = bound_boss(12236, 349, 2);
        let player = h.player();
        let origin = *h.app.world().get::<Position>(boss).unwrap();
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let reach = super::super::super::model_reach::combat_reach(h.app.world(), boss).unwrap()
            + super::super::super::model_reach::combat_reach(h.app.world(), player).unwrap();
        // Below the reach-adjusted minimum, or exactly at the strict3D maximum.
        let outside = if vertical {
            Position {
                y: origin.y + 30.0 + reach,
                ..origin
            }
        } else {
            Position {
                x: origin.x + 5.0 + reach - 0.5,
                ..origin
            }
        };
        h.app.world_mut().entity_mut(player).insert((
            outside,
            Health {
                current: 1_000_000.0,
                max: 1_000_000.0,
            },
        ));
        h.engage(boss);
        h.advance(0.05);
        assert!(
            spell_gos(&h, boss, 21390).is_empty(),
            "range-excluded victim"
        );
        let current = *h.app.world().get::<Position>(boss).unwrap();
        h.app.world_mut().entity_mut(player).insert(Position {
            x: current.x + 10.0,
            ..current
        });
        h.advance(0.45);
        assert!(
            spell_gos(&h, boss, 21390).is_empty(),
            "source500ms recheck must not run early"
        );
        h.advance(0.1);
        assert_eq!(
            spell_gos(&h, boss, 21390).len(),
            1,
            "eligible victim after500ms recheck"
        );
    }
}

#[test]
fn encounter_coverage_vyletongue_flash_stuns_a_real_nearby_victim() {
    let (mut h, boss, _) = bound_boss(12236, 349, 2);
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
    let mut stun_seen = false;
    for _ in 0..300 {
        h.advance(0.05);
        stun_seen |= h.has_aura(player, 7964);
    }
    assert!(stun_seen, "real source7964 nearby stun, no aura injection");
}
