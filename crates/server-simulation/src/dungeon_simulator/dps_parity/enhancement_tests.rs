//! The parity actor must execute the pinned Enhancement APL, not a diagnostic priority.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_literal_actor_executes_stormstrike_instead_of_recasting_weapon_imbue() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(263, 1).unwrap();
    engage(&mut run);
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::CastSuccess
            })
            .count()
    };
    assert!(casts(17364) > 0, "literal single_sb must cast Stormstrike");
    assert_eq!(casts(33757), 1, "Windfury Weapon belongs to precombat only");
}
