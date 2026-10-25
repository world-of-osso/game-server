//! Real selected Crusading swing funds the native Holy Power pool.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "actual Crusading Strike hit must grant native406834 Holy Power"]
fn dps_parity_ret_crusading_actual_swing_grants_holy_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(has(&run, 404542));
    assert_eq!(current_holy_power(&run), 0.0);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    for _ in 0..400 {
        run.app.update();
        let hit = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(408385)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        if hit {
            assert_eq!(
                current_holy_power(&run),
                1.0,
                "first real successful Crusading Strike earns native406834 effect0"
            );
            return;
        }
    }
    panic!("actual selected Aura361 swing must deal408385 damage first");
}
