//! Installed Razorlash SAI; controlled survival, not ordinary-fight/native acceptance.
use super::*;

#[test]
fn encounter_coverage_razorlash_source_spells_damage_and_isolated_credit() {
    const BOSS: u32 = 12258;
    const BIT: u32 = 1;
    const ENCOUNTER: u32 = 423;
    let (mut h, boss, map) = bound_boss(BOSS, 349, BIT);
    let player = h.player();
    // Survival staging only. Source70%/85% chance, resources, flags and timers stay intact.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    let mut thrash_seen = false;
    let mut puncture_seen = false;
    for tick in 1..=3600 {
        h.advance(0.05);
        let gos = &h.inbox(0).spell_gos;
        casts.extend(gos[cursor..].iter().filter_map(|go| {
            (go.caster == boss.to_bits() && [12787, 40504, 15976].contains(&go.spell_id))
                .then_some((go.spell_id, tick as f32 * 0.05, go.target))
        }));
        cursor = gos.len();
        thrash_seen |= h.has_aura(boss, 12787);
        puncture_seen |= h.has_aura(player, 15976);
    }
    eprintln!("RAZORLASH successful source casts: {casts:?}");
    for (spell, first, repeat, target) in [
        (12787, 4.0, 6.5, boss),
        (40504, 6.0, 8.0, player),
        (15976, 12.0, 12.0, player),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _, _)| *id == spell)
            .map(|(_, time, _)| *time)
            .collect();
        assert!(
            times.len() >= 2,
            "missing successful repeat {spell}: {times:?}"
        );
        // TC SmartScript.cpp3794 uses timer < diff. Each repeat consumes one extra50ms tick.
        // Chance failures skip whole windows without changing the authored interval.
        let window_interval = repeat + 0.05;
        for time in &times {
            let window = ((*time - first) / window_interval).round();
            assert!(
                window >= 0.0 && (*time - (first + window * window_interval)).abs() <= 0.1,
                "off source timer window {spell}: {times:?}"
            );
        }
        assert!(
            casts
                .iter()
                .filter(|(id, _, _)| *id == spell)
                .all(|(_, _, actual)| *actual == Some(target.to_bits())),
            "source target {spell}"
        );
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
    }
    assert!(thrash_seen, "real self Thrash aura must apply");
    assert!(puncture_seen, "real victim Puncture aura must apply");
    for spell in [40504, 15976] {
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "real damage {spell}"
        );
    }
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
    let before = spell_gos(&h, boss, 40504).len();
    h.advance(15.0);
    assert_eq!(
        spell_gos(&h, boss, 40504).len(),
        before,
        "dead boss must not cast"
    );
}
