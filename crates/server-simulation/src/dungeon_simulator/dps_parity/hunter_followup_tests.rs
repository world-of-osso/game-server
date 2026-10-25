//! Native MM action cadence and the missing Sentinel/residual producers.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use super::*;
use shared::protocol::CombatLogKind;

fn owns_target_aura(run: &Run, spell: u32) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .any(|aura| aura.caster == run.actor && aura.spell_id == spell)
}

fn damage_events(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn aimed_full_recharge(run: &Run) -> f64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(19434)
        .unwrap();
    crate::spell_cast::cooldown_query::cooldown_value(world, run.actor, spell, "full_recharge_time")
        .unwrap()
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_rapid_ticks_reduce_actual_aimed_recharge_with_take_aim() {
    // SimC76f8118 sc_hunter.cpp5686–5687: each actual Rapid Fire tick
    // executes Take Aim's native effect2 reduction, even without a crit.
    let mut run = player(1);
    cast(&mut run, 19434, false);
    cast(&mut run, 19434, false);
    let before = aimed_full_recharge(&run);
    let start = crate::spell_cast::history_now(run.app.world());
    cast(&mut run, 257044, false);
    let elapsed = crate::spell_cast::history_now(run.app.world()) - start;
    let reduction = before - elapsed - aimed_full_recharge(&run);
    let ticks = damage_events(&run, 257045);
    let take_aim = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1273132)
        .unwrap();
    let seconds_per_tick = f64::from(
        take_aim
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .base_points,
    ) / 1000.0;
    assert!(
        ticks > 0 && seconds_per_tick > 0.0,
        "native Take Aim and actual Rapid impacts"
    );
    assert!(
        (reduction - ticks as f64 * seconds_per_tick).abs() < 0.1,
        "real Aimed recharge loses the native amount per Rapid tick: reduction={reduction}, ticks={ticks}, seconds={seconds_per_tick}"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_auto_shot_produces_selected_lock_and_load() {
    // SimC76f8118 sc_hunter.cpp3991/4014–4016: native talent chance
    // rolls on the actual repeating Auto Shot, not every successful spell.
    let _seed = crate::random::seed_scope(70_448);
    let mut run = player(1);
    cast(&mut run, 75, false);
    for _ in 0..6000 {
        run.app.update();
        if owned_buff(&run, 194594) {
            return;
        }
    }
    panic!("actual Auto Shots never grant selected Lock and Load");
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_feathered_trueshot_mark_is_consumed_into_lunar_storm() {
    // SimC76f8118 sc_hunter.cpp6910–6911/3654–3666/5422–5428.
    let mut run = player(1);
    cast(&mut run, 288613, true);
    assert!(
        owns_target_aura(&run, 1253601),
        "actual Trueshot force-produces Sentinel's Mark at its hostile primary"
    );
    cast(&mut run, 19434, false);
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        !owns_target_aura(&run, 1253601),
        "actual Aimed impact consumes the mark"
    );
    assert!(
        damage_events(&run, 1253733) > 0,
        "the consumed mark emits native Lunar Storm damage"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_rapid_fire_damage_banks_sanctified_armaments() {
    // SimC76f8118 sc_hunter.cpp5708–5711: actual Rapid impact feeds
    // the native residual1253836; not a fabricated aura in the test.
    let mut run = player(1);
    cast(&mut run, 257044, false);
    for _ in 0..160 {
        run.app.update();
    }
    assert!(
        damage_events(&run, 257045) > 0,
        "actual Rapid Fire damaged the target"
    );
    assert!(
        damage_events(&run, 1253836) > 0,
        "Rapid Fire produces actual Sanctified Armaments ticks"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_aimed_crit_marks_for_later_blood_fletching_damage() {
    // SimC76f8118 sc_hunter.cpp5411–5419/8630–8657: the applying
    // critical Aimed hit cannot trigger its own mark's damage.
    let _seed = crate::random::seed_scope(70_448);
    let mut run = player(1);
    for _ in 0..8 {
        cast(&mut run, 19434, false);
        for _ in 0..30 {
            run.app.update();
        }
        if owns_target_aura(&run, 1319016) {
            break;
        }
    }
    assert!(
        owns_target_aura(&run, 1319016),
        "actual Aimed crit applies native Blood Fletching mark"
    );
    let before = damage_events(&run, 1319015);
    cast(&mut run, 75, false);
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        damage_events(&run, 1319015) > before,
        "a later native attack triggers Blood Fletching damage"
    );
}
