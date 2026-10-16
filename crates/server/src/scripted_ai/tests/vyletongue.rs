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
    let mut stun_seen = false;
    let mut blink_seen = false;
    for tick in 1..=1300 {
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
                assert!(
                    displacement > 10.0,
                    "actual Blink displacement: {displacement}"
                );
                blink_seen = true;
            }
        }
        cursor = gos.len();
        stun_seen |= h.has_aura(player, 7964);
    }
    eprintln!("VYLETONGUE source casts={casts:?}, stun={stun_seen}, blink={blink_seen}");
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
    for pair in range_times.windows(2) {
        let interval = pair[1] - pair[0];
        assert!(
            (7.0..=9.6).contains(&interval),
            "source range repeat7–9.5s: {range_times:?}"
        );
    }
    for (spell, minimum, maximum) in [(16100, 0.0, 0.1), (7964, 8.0, 12.6), (21655, 20.0, 30.1)] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time)
            .collect();
        assert!(times.len() >= 2, "source repeated {spell}: {times:?}");
        assert!(
            times[0] >= minimum && times[0] <= maximum,
            "source first {spell}: {times:?}"
        );
    }
    assert!(stun_seen, "real Flash7964 player stun");
    assert!(blink_seen, "real21655 displacement");
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
