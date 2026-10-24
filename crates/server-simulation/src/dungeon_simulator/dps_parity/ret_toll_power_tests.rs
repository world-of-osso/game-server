//! Separate funding qualification; dispatch relationship does not invent220637.
use super::ret_purpose_tests::current_holy_power;
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TOLL: u32 = 375576;
const JUDGMENT_POWER: u32 = 220637;
const JUDGMENT_RANK: u32 = 315867;
const HAMMER: u32 = 1279408;
const HAMMER_MODE: u32 = 1277026;

fn qualify_judgment_funding(count: usize) {
    let run = super::ret_toll_tests::qualify_toll(count);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(JUDGMENT_RANK),
        "native tooltip's learned rank must be present"
    );
    let power = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(JUDGMENT_POWER)
        .unwrap();
    let amount = power
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(amount, 1.0);
    assert_eq!(
        current_holy_power(&run),
        amount * count as f32,
        "each actually damaging native Toll Judgment must grant its sourced220637 amount"
    );
}

#[test]
#[ignore = "separate Toll funding: actual ST406957 must grant native220637 with learned315867"]
fn dps_parity_ret_toll_power_native_single_judgment_grants_actual_resource() {
    qualify_judgment_funding(1);
}

#[test]
#[ignore = "separate Toll funding: three actual406957 children must grant separate native resources"]
fn dps_parity_ret_toll_power_native_three_judgments_grant_actual_resources() {
    qualify_judgment_funding(3);
}

#[test]
#[ignore = "native Hammer-mode Toll control already contains actual Energize effect1"]
fn dps_parity_ret_toll_power_native_hammer_child_uses_its_own_energize() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_eq!(current_holy_power(&run), 0.0);
    let native = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(HAMMER)
        .unwrap();
    let amount = native
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    assert_eq!(amount, 1.0);
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        HAMMER_MODE,
        Some(run.actor),
        0,
    );
    assert!(has(&run, HAMMER_MODE));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        TOLL,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..200 {
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
        "real native Hammer child must actually hit"
    );
    assert_eq!(
        current_holy_power(&run),
        amount,
        "native Hammer effect1 must not get duplicate Judgment funding"
    );
}
