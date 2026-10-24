//! Bounded native Toll cap/range/combat-admission and mode/percentage controls.
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

const TOLL: u32 = 375576;
const JUDGMENT: u32 = 406957;
const HAMMER: u32 = 1279408;
const HAMMER_MODE: u32 = 1277026;

fn cast_toll(run: &mut Run) {
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
}

fn child_targets(run: &Run, child: u32) -> Vec<u64> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(child)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect()
}

#[test]
#[ignore = "native Toll cap5 preserves primary and excludes real far/out-of-combat targets"]
fn dps_parity_ret_toll_control_native_cap_range_and_combat_admission() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(TOLL)
        .unwrap();
    let cap = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as usize;
    let area = spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap();
    let radius = crate::spell_cast::targets::effect_radius(area);
    assert_eq!(cap, 5);
    assert!(radius > 0.0);
    for index in 1..cap + 2 {
        let target = spawn_dummy(&mut run.app, run.actor, index).unwrap();
        run.targets.push(target);
    }
    let near: BTreeSet<_> = run.targets.iter().map(|target| target.to_bits()).collect();
    let idle = spawn_dummy(&mut run.app, run.actor, cap + 2).unwrap();
    let far = crate::zones::summon_creature(
        run.app.world_mut(),
        run.actor,
        DUMMY_ENTRY,
        [radius * 2.0, 0.0, 0.0, 0.0],
    )
    .unwrap();
    run.targets.push(far);
    engage_fixture_opponents(&mut run);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(
        !run.app
            .world()
            .get::<shared::components::CombatStatus>(idle)
            .is_some_and(|status| status.0)
    );
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(far)
            .unwrap()
            .0
    );
    cast_toll(&mut run);
    for _ in 0..200 {
        run.app.update();
        if child_targets(&run, JUDGMENT).len() >= cap {
            break;
        }
    }
    let recipients = child_targets(&run, JUDGMENT);
    assert_eq!(
        recipients.len(),
        cap,
        "native cap counts total child dispatch, not extra targets"
    );
    let unique: BTreeSet<_> = recipients.into_iter().collect();
    assert_eq!(
        unique.len(),
        cap,
        "explicit primary must not be dispatched twice"
    );
    assert!(unique.contains(&run.targets[0].to_bits()));
    assert!(unique.is_subset(&near));
    assert!(!unique.contains(&idle.to_bits()));
    assert!(!unique.contains(&far.to_bits()));
}

#[test]
#[ignore = "actual native1277026 mode selects Hammer child and real native effect5 percentage"]
fn dps_parity_ret_toll_control_native_hammer_mode_and_damage_percentage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    // Isolate the sourced relationship; this does not qualify its mode producer.
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        HAMMER_MODE,
        Some(run.actor),
        0,
    );
    assert!(has(&run, HAMMER_MODE));
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, TOLL, 5);
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            HAMMER,
            "divine_toll_mode",
            8,
        ),
    );
    cast_toll(&mut run);
    for _ in 0..200 {
        run.app.update();
        let has_damage = run
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
            });
        if has_damage {
            break;
        }
    }
    assert_eq!(child_targets(&run, HAMMER), vec![run.targets[0].to_bits()]);
    assert!(child_targets(&run, JUDGMENT).is_empty());
    let captured = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let result = captured["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == HAMMER)
        .expect("actual Hammer child damage must expose its native result stage");
    let factors = &result["factors"];
    let measured = factors["total_amount"].as_f64().unwrap()
        / (factors["native_source_amount"].as_f64().unwrap()
            * factors["target_product"].as_f64().unwrap());
    let expected = 1.0 + f64::from(percent) / 100.0;
    assert!(
        (measured - expected).abs() <= 4.0 * f64::from(f32::EPSILON),
        "actual damage-stage factor{measured} must equal native Toll percentage{expected}"
    );
}
