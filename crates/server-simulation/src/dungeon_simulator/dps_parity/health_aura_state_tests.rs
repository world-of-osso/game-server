//! Real31146/ordinary native24275 admission consumes generic health state2.
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const HAMMER: u32 = 24275;

fn fixture() -> Run {
    let run = native_ret(4);
    assert_eq!(
        literal_rotation::execution::resolve_action(run.app.world(), run.actor, HAMMER).unwrap(),
        Some(HAMMER)
    );
    assert!(!has(&run, 31884));
    assert!(!has(&run, 454351));
    let state = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HAMMER)
        .unwrap()
        .aura_restrictions
        .as_ref()
        .unwrap()
        .target_aura_state;
    assert_eq!(state, 2, "native24275 requires TC WOUNDED_20_PERCENT");
    run
}

#[test]
#[ignore = "native real31146 full health must fail24275's generic health aura state2"]
fn health_aura_state_native_full_health_dummy_rejects_hammer() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let health = run.app.world().get::<Health>(run.targets[0]).unwrap();
    assert_eq!(health.current, health.max);
    let result = crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        HAMMER,
        Some(run.targets[0]),
        false,
    );
    assert_eq!(result, Err(CastFailure::TargetAuraState));
}

#[test]
#[ignore = "native real31146 below20percent must admit actual24275 and deal real damage"]
fn health_aura_state_native_wounded_dummy_accepts_hammer() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    // Threshold-state fixture only, not damage calibration or a DPS measurement.
    // Keep the real native template/max health, spell metadata, book and powers.
    let mut health = run
        .app
        .world_mut()
        .get_mut::<Health>(run.targets[0])
        .unwrap();
    health.current = (health.max * 0.19).trunc();
    assert!(health.current > 0.0);
    drop(health);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        HAMMER,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
        if run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(HAMMER)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            break;
        }
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(HAMMER)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "ordinary native Hammer must actually damage the wounded real31146"
    );
}
