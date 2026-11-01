//! The class-party Discipline follower casts its selected Ultimate Penitence.

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_follower_channels_ultimate_penitence_at_the_enemy() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(1);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer, enemy) = (run.tank, run.healer, run.sources[0]);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(healer)
            .is_some_and(|known| known.contains(421453)),
        "selected loadout entry 116182 must teach Ultimate Penitence"
    );
    super::healer_simc_replay::finish_precombat(&mut run.app, &[tank, healer]).unwrap();
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    // 40 s at 20 ticks per second.
    for _ in 0..800 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    let from_healer = |spell, kind| {
        events
            .iter()
            .filter(move |event| {
                event.source == Some(healer.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == kind
            })
            .collect::<Vec<_>>()
    };
    let casts = from_healer(421453, CombatLogKind::CastSuccess);
    assert_eq!(casts.len(), 1, "one cast per 240 s cooldown");
    assert_eq!(casts[0].target, Some(enemy.to_bits()));
    let bolts = from_healer(421543, CombatLogKind::Damage);
    assert!(
        bolts.len() >= 20 && bolts.iter().all(|e| e.target == Some(enemy.to_bits())),
        "actual channel bolts hit the enemy: {bolts:?}"
    );
}
