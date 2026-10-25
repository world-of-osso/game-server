//! Pinned Celebras source combat, corruption summons and redeemed death actor.
use super::*;

/// Feasible due-time interval, not previous dispatch: TaskContext::Repeat anchors
/// to due time even when the casting validator deferred that dispatch.
pub(super) fn follows_task_deadlines(
    times: &[f32],
    first: (f32, f32),
    repeat: (f32, f32),
    blocked: &[(f32, bool)],
) -> bool {
    if times.len() < 2 {
        return false;
    }
    let mut due = (0.0_f32, 0.0_f32);
    let mut previous_dispatch = 0.0;
    for (index, dispatch) in times.iter().copied().enumerate() {
        let interval = if index == 0 { first } else { repeat };
        let last_unblocked = blocked
            .iter()
            .filter(|(tick, casting)| {
                !casting && *tick > previous_dispatch + 0.1 && *tick < dispatch - 0.1
            })
            .map(|(tick, _)| *tick)
            .last()
            .unwrap_or(previous_dispatch);
        due = (
            (due.0 + interval.0).max(last_unblocked - 0.1),
            (due.1 + interval.1).min(dispatch + 0.1),
        );
        if due.0 > due.1 {
            return false;
        }
        previous_dispatch = dispatch;
    }
    true
}

#[test]
fn encounter_coverage_celebras_due_clock_observer_controls() {
    let casting: Vec<_> = (1..=160).map(|tick| (tick as f32 * 0.05, true)).collect();
    assert!(follows_task_deadlines(
        &[0.05, 8.0, 10.0],
        (0.0, 0.0),
        (5.0, 5.0),
        &casting
    ));
    assert!(!follows_task_deadlines(
        &[0.05, 4.0],
        (0.0, 0.0),
        (5.0, 5.0),
        &casting
    ));
    let unblocked: Vec<_> = casting.iter().map(|(tick, _)| (*tick, false)).collect();
    assert!(!follows_task_deadlines(
        &[0.05, 8.0],
        (0.0, 0.0),
        (5.0, 5.0),
        &unblocked
    ));
}

#[test]
fn encounter_coverage_celebras_source_spells_summons_and_isolated_credit() {
    const BIT: u32 = 3;
    const ENCOUNTER: u32 = 425;
    let (mut h, boss, map) = bound_boss(12225, 349, BIT);
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
    let mut roots_seen = false;
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut blocked = Vec::new();
    for tick in 1..=1300 {
        blocked.push((
            (tick - 1) as f32 * 0.05,
            h.app.world().get::<CastState>(boss).is_some(),
        ));
        h.advance(0.05);
        roots_seen |= h.has_aura(player, 12747);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [21807, 12747, 21968].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
    }
    eprintln!("CELEBRAS source casts={casts:?}");
    for (spell, first, repeat, cast_time, target) in [
        (21807, (0.0, 6.0), (4.0, 8.0), 2.0, player),
        (12747, (0.0, 5.0), (20.0, 20.0), 1.5, player),
        (21968, (30.0, 30.0), (20.0, 20.0), 0.0, boss),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time - cast_time)
            .collect();
        assert!(
            follows_task_deadlines(&times, first, repeat, &blocked),
            "source deadlines {spell}: {times:?}"
        );
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, actual)| *actual == Some(target.to_bits())),
            "source target{spell}"
        );
    }
    for spell in [21807, 12747, 21968] {
        assert!(
            !spell_gos(&h, boss, spell).is_empty(),
            "source Celebras spell{spell}"
        );
    }
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(21807)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real Wrath damage"
    );
    assert!(roots_seen, "real victim Entangling Roots");
    let adds = h.creatures_of(13743);
    assert_eq!(
        adds.len(),
        6,
        "two actual21968 waves of three Corrupt Forces"
    );
    for add in &adds {
        assert_eq!(*h.app.world().get::<InMap>(*add).unwrap(), map);
        assert_eq!(
            h.app
                .world()
                .get::<crate::zones::TempSummon>(*add)
                .unwrap()
                .summoner,
            boss
        );
    }
    let death_position = *h.app.world().get::<Position>(boss).unwrap();
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(
        h.creatures_of(13743).is_empty(),
        "death cleans corruption summons"
    );
    let redeemed = h.creatures_of(13716);
    assert_eq!(
        redeemed.len(),
        1,
        "source death creates Celebras the Redeemed"
    );
    let redeemed = redeemed[0];
    assert_eq!(*h.app.world().get::<InMap>(redeemed).unwrap(), map);
    let position = h.app.world().get::<Position>(redeemed).unwrap();
    assert!(
        Vec3::new(
            position.x - death_position.x,
            position.y - death_position.y,
            position.z - death_position.z
        )
        .length()
            < 0.01,
        "redeemed at death position"
    );
    assert!(
        (h.app.world().get::<Rotation>(redeemed).unwrap().y - std::f32::consts::FRAC_PI_2).abs()
            < 0.001,
        "source redeemed WoW orientation0"
    );
    let template = &h.app.world().resource::<WorldData>().templates[&13716];
    assert_eq!(
        template.health_modifier, 35.0,
        "TDB base redeemed health, not legacy8"
    );
    assert!(
        (template.mana_modifier - 5.3615).abs() < 0.0001,
        "TDB base redeemed mana, not legacy4"
    );
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
    assert!(
        h.inbox(1)
            .spell_gos
            .iter()
            .all(|go| go.caster != boss.to_bits())
    );
    let casts_before = spell_gos(&h, boss, 21807).len()
        + spell_gos(&h, boss, 12747).len()
        + spell_gos(&h, boss, 21968).len();
    h.advance(30.0);
    assert_eq!(
        spell_gos(&h, boss, 21807).len()
            + spell_gos(&h, boss, 12747).len()
            + spell_gos(&h, boss, 21968).len(),
        casts_before,
        "dead source caster stays silent"
    );
    assert_eq!(
        h.creatures_of(13716),
        vec![redeemed],
        "manual-despawn redeemed survives corpse cleanup"
    );
}
