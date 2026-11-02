//! A class follower heals its injured owner between pulls.
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_shaman_heals_injured_owner_out_of_combat() {
    let mut run = super::healer_simc_replay::snapshot_party(264, 70, 1);
    let owner = run.slots[4];
    let healer = run.healer;
    let health = run.app.world_mut().get_mut::<Health>(owner).unwrap();
    let initial = health.max * 0.666;
    run.app
        .world_mut()
        .get_mut::<Health>(owner)
        .unwrap()
        .current = initial;

    for _ in 0..600 {
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        if events.iter().any(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.target == Some(owner.to_bits())
                && event.amount > 0
        }) {
            assert!(run.app.world().get::<Health>(owner).unwrap().current > initial);
            return;
        }
    }
    panic!("idle Restoration Shaman did not heal its injured owner");
}
