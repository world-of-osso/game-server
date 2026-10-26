//! Distinct Storm-gap boundaries: native Aura361 and sourced Second Sunrise.
use super::ret_purpose_tests::{current_holy_power, earn_finisher_power};
use super::ret_tier_tests::{has, native_ret};
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::{
    CastFailure, CastTargets,
    test_harness::{ObservedCombatEvents, ObservedTriggeredCasts},
};
use shared::protocol::CombatLogKind;

const WAKE: u32 = 255937;
const STORM: u32 = 53385;
const SUNRISE: u32 = 431474;
const CRUSADING_STRIKES: u32 = 404542;
const CRUSADING_DAMAGE: u32 = 408385;

fn start_attack(run: &mut Run) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
}

fn damage(run: &Run, spell: Option<u32>) -> i64 {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == spell
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

fn cast_ordinary(run: &mut Run, spell: u32, unit: Entity) {
    for _ in 0..200 {
        let result = crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            spell,
            CastTargets {
                unit,
                primary: Some(run.targets[0]),
                destination: None,
            },
        );
        match result {
            Ok(()) => return,
            Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
            Err(reason) => panic!(
                "ordinary native {spell} failed: {reason:?}; Wake damage={}, Storm damage={}, HolyPower={}",
                damage(run, Some(WAKE)),
                damage(run, Some(STORM)),
                current_holy_power(run)
            ),
        }
    }
    panic!("ordinary native {spell} never became ready");
}

fn wait_for_funding(run: &mut Run) {
    for _ in 0..100 {
        if current_holy_power(run) >= 3.0 {
            return;
        }
        run.app.update();
    }
    panic!("actual Wake must fund Storm");
}

fn cast_funded_storms(run: &mut Run, count: usize) -> usize {
    let mut paid = 0;
    let target = run.targets[0];
    let actor = run.actor;
    cast_ordinary(run, WAKE, target);
    wait_for_funding(run);
    for _ in 0..count {
        earn_finisher_power(run, STORM);
        let before = current_holy_power(run);
        cast_ordinary(run, STORM, actor);
        paid += usize::from(before - current_holy_power(run) >= 3.0);
        for _ in 0..40 {
            run.app.update();
        }
    }
    assert!(damage(run, Some(WAKE)) > 0, "actual Wake must damage");
    assert!(damage(run, Some(STORM)) > 0, "actual Storm must damage");
    assert!(paid > 0, "at least one ordinary Storm must pay native3HP");
    paid
}

fn echo_targets(run: &Run) -> Vec<Option<Entity>> {
    run.app
        .world()
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(caster, spell, _)| *caster == run.actor && *spell == STORM)
        .map(|(_, _, target)| *target)
        .collect()
}

fn assert_live_sunrise(run: &Run) {
    assert!(has(run, SUNRISE));
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SUNRISE && aura.caster == run.actor)
        .unwrap();
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount
    };
    assert_eq!(amount(0), 20.0, "native15 plus actual1258011 modifier5");
    assert_eq!(amount(1), 50.0, "native30 plus actual1258011 modifier20");
}

pub(super) fn remove_hero_selection(run: &mut Run) {
    let (_, mut traits) = read_reference(70).unwrap();
    let classes = run.app.world().resource::<crate::spell_info::ClassSpells>();
    let tree = &classes
        .classes
        .iter()
        .find(|class| class.class_id == 2)
        .unwrap()
        .traits;
    traits.retain(|entry| {
        let node = tree.node(entry.node_id).unwrap();
        node.sub_tree_id == 0 && node.kind != crate::spell_info::TraitNodeType::SubTreeSelection
    });
    inputs::install_traits(run.app.world_mut(), run.actor, &traits).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!has(run, SUNRISE));
}

#[test]
#[ignore = "selected native Aura361 must replace a real swing through its TriggerSpell"]
fn dps_parity_ret_storm_gap_aura361_replaces_actual_white_swing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert!(has(&run, CRUSADING_STRIKES));
    start_attack(&mut run);
    for _ in 0..400 {
        run.app.update();
        if damage(&run, Some(CRUSADING_DAMAGE)) > 0 {
            break;
        }
    }
    let white = damage(&run, None);
    let replacement = damage(&run, Some(CRUSADING_DAMAGE));
    assert!(white + replacement > 0, "a real swing must damage first");
    assert!(
        replacement > 0,
        "native Aura361 must launch408385; actual white damage={white}"
    );
    assert_eq!(white, 0, "replacement must not duplicate white damage");
}

#[test]
#[ignore = "native unselected auto-attack control retains ordinary white swings"]
fn dps_parity_ret_storm_gap_no_override_retains_actual_white_swing() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!has(&run, CRUSADING_STRIKES));
    start_attack(&mut run);
    for _ in 0..400 {
        run.app.update();
        if damage(&run, None) > 0 {
            break;
        }
    }
    assert!(
        damage(&run, None) > 0,
        "real white swing control must damage"
    );
    assert_eq!(damage(&run, Some(CRUSADING_DAMAGE)), 0);
}

#[test]
#[ignore = "selected live Second Sunrise and real funded Storms must produce a native background echo"]
fn dps_parity_ret_storm_gap_selected_sunrise_launches_real_echo() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_live_sunrise(&run);
    run.app.init_resource::<ObservedTriggeredCasts>();
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        STORM,
        "native_storm_and_sunrise",
        256,
    ));
    start_attack(&mut run);
    let paid = cast_funded_storms(&mut run, 64);
    let targets = echo_targets(&run);
    assert!(
        !targets.is_empty(),
        "64 ordinary naturally funded Storms ({paid} paid) with native20% Sunrise produced no echo"
    );
    assert!(
        targets.iter().all(|target| *target == Some(run.targets[0])),
        "background echo must retain the real hostile primary, not the self cast unit"
    );
    let capture = run.app.world().resource::<FactorCapture>().export();
    let events = capture["events"].as_array().unwrap();
    let amounts = events
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == STORM);
    let tempest_scale = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        383396,
        0,
    )) / 100.0;
    let mut root_hits = 0;
    let mut echo_hits = 0;
    let mut tempest_hits = 0;
    for event in amounts {
        let scale = event["applied_multipliers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["factor"] == "cast_effectiveness")
            .unwrap()["value"]
            .as_f64()
            .unwrap();
        // SimC76f8118 Ret495–532: Sunrise's Tempest is also53385,
        // but is a separate secondary child, not another50% Sunrise echo.
        match event["native_depth"].as_u64().unwrap() {
            0 => {
                assert_eq!(scale, 1.0, "root amount must remain native");
                root_hits += 1;
            }
            1 => {
                assert_eq!(scale, 0.5, "actual echo damage must use live native50%");
                echo_hits += 1;
            }
            2 => {
                assert!((scale - tempest_scale).abs() < 1e-6);
                tempest_hits += 1;
            }
            depth => panic!("unexpected Storm child depth{depth}: {event}"),
        }
    }
    assert!(
        root_hits > 0 && echo_hits > 0 && tempest_hits > 0,
        "real root, echo and same-ID Tempest damage required: {capture}"
    );
    println!(
        "STORM_ECHO_PROOF root_hits={root_hits} echo_hits={echo_hits} tempest_hits={tempest_hits} paid={paid} launches={}",
        targets.len()
    );
}

#[test]
#[ignore = "bounded ordinary-root Sunrise policy must not recurse from unrelated background Storms"]
fn dps_parity_ret_storm_gap_background_sunrise_control_does_not_recurse_or_pay() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    assert_live_sunrise(&run);
    run.app.init_resource::<ObservedTriggeredCasts>();
    start_attack(&mut run);
    let target = run.targets[0];
    cast_ordinary(&mut run, WAKE, target);
    wait_for_funding(&mut run);
    for _ in 0..12 {
        let power = current_holy_power(&run);
        crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, STORM, Some(target), 1);
        // SimC76f8118 Ret504–507: background Storm costs zero. Compare
        // at launch, before independent Crusading Strikes can earn more power.
        assert_eq!(current_holy_power(&run), power);
        for _ in 0..40 {
            run.app.update();
        }
    }
    assert!(
        damage(&run, Some(STORM)) > 0,
        "background control must actually damage"
    );
    assert_eq!(
        echo_targets(&run).len(),
        12,
        "only the12requested native launches, no recursive child"
    );
}

#[test]
#[ignore = "legal unspent Hero points cannot create a Second Sunrise echo"]
fn dps_parity_ret_storm_gap_no_sunrise_keeps_echo_absent() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = native_ret(4);
    remove_hero_selection(&mut run);
    run.app.init_resource::<ObservedTriggeredCasts>();
    start_attack(&mut run);
    cast_funded_storms(&mut run, 4);
    assert!(echo_targets(&run).is_empty());
}
