use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires the level-90 follower snapshot"]
fn mistweaver_uses_fistweaving_below_six_enemies_and_sck_at_six() {
    for enemy_count in [1, 2, 5, 6] {
        let mut party = super::super::healer_simc_replay::snapshot_party(270, 90, enemy_count);
        super::super::healer_simc_replay::engage(&mut party);
        let healer = party.healer;
        let enemy = party.sources[0];
        party
            .app
            .world_mut()
            .entity_mut(party.tank)
            .insert(crate::networking::CombatTarget(Some(enemy)));
        for _ in 0..300 {
            party.app.update();
        }
        let casts: Vec<_> = party
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(healer.to_bits()) && event.kind == CombatLogKind::CastSuccess
            })
            .filter_map(|event| event.spell_id)
            .collect();
        if enemy_count < 6 {
            assert!(
                !casts.contains(&101546),
                "SCK must wait for six enemies: {enemy_count} {casts:?}"
            );
            assert!(
                casts.contains(&100784),
                "Blackout Kick must be used with {enemy_count} enemies: {casts:?}"
            );
            assert!(
                casts.contains(&100780),
                "Tiger Palm must be used with {enemy_count} enemies: {casts:?}"
            );
        } else {
            assert!(
                casts.contains(&101546),
                "SCK must be available with six enemies: {casts:?}"
            );
        }
    }
}
