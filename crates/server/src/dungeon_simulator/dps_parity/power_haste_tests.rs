//! Native PowerType0x400 resource regeneration, not fabricated haste or pools.
use super::*;
use crate::auras::AuraStatMods;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};
use crate::spell_cast::test_harness::{Harness, harness};
use server::simc_apl::Facts;
use shared::components::{Mana, PowerEntry, PowerType, UnitPowers};

const REGEN_AFFECTED_BY_HASTE: i64 = 0x0400;
const COBRA_SHOT: u32 = 193455;
const TIGER_PALM: u32 = 100780;
const FLASH_OF_LIGHT: u32 = 19750;

#[derive(Debug, PartialEq)]
struct ResourceSnapshot {
    current: f32,
    maximum: f32,
    cached_rate: f32,
    wire: PowerEntry,
}

fn resource_name(power: PowerType) -> &'static str {
    match power {
        PowerType::Focus => "focus",
        PowerType::Energy => "energy",
        PowerType::Mana => "mana",
        _ => panic!("fixture does not model {power:?}"),
    }
}

fn cached_rate(run: &Run, power: PowerType) -> f32 {
    let pools = run.app.world().get::<PowerPools>(run.actor).unwrap();
    if power == PowerType::Mana {
        pools.mana_regen_per_sec
    } else {
        pools.get(power).unwrap().regen_per_sec
    }
}

fn resource_snapshot(run: &Run, power: PowerType) -> ResourceSnapshot {
    let world = run.app.world();
    let (current, maximum) =
        crate::spell_cast::powers::power_value(world, run.actor, power).unwrap();
    let wire = world
        .get::<UnitPowers>(run.actor)
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.power == power)
        .unwrap()
        .clone();
    ResourceSnapshot {
        current,
        maximum,
        cached_rate: cached_rate(run, power),
        wire,
    }
}

fn read_expected_native_regen(run: &Run, power: PowerType) -> f32 {
    let world = run.app.world();
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    let mods = world.get::<AuraStatMods>(run.actor).unwrap();
    let (base, haste_factor) = if power == PowerType::Mana {
        (stats.base_mana * 0.01, 1.0)
    } else {
        let info = world.resource::<PowerTypeTable>().get(power).unwrap();
        (info.regen_combat, 1.0 + stats.haste_pct / 100.0)
    };
    let multiplier = mods.power_regen(power as i32);
    let flat = mods.power_regen_flat(power as i32) / 5.0;
    println!(
        "NATIVE_REGEN_INPUT spec={} power={power:?} haste_pct={} base={base} multiplier={multiplier} haste_factor={haste_factor} flat={flat}",
        run.profile.spec_id, stats.haste_pct
    );
    base * multiplier * haste_factor + flat
}

fn read_resource_facts(run: &Run, power: PowerType) -> (f64, f64) {
    let name = resource_name(power);
    let bindings = Bindings {
        resources: [(name.into(), power)].into(),
        ..Default::default()
    };
    let world = run.app.world();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(world, run.actor, &bindings, encounter, 1);
    (
        facts.value(&format!("{name}.regen")).unwrap(),
        facts.value(&format!("{name}.time_to_max")).unwrap(),
    )
}

fn assert_read_only_potential_facts(run: &Run, power: PowerType, expected: f32) {
    assert!(
        expected > 0.0,
        "real fixture has positive native potential regen"
    );
    let before = resource_snapshot(run, power);
    let table = run.app.world().resource::<PowerTypeTable>();
    let display = f64::from(table.get(power).unwrap().display_modifier);
    let expected_displayed = f64::from(expected) / display;
    let expected_time = f64::from(before.maximum - before.current) / f64::from(expected);
    let tolerance = f64::from(f32::EPSILON * expected.abs().max(1.0) * 8.0);
    for _ in 0..2 {
        let (actual_rate, actual_time) = read_resource_facts(run, power);
        assert!(
            (actual_rate - expected_displayed).abs() <= tolerance,
            "native {power:?} potential fact{actual_rate}, expected{expected_displayed}, current{}, max{}, cached{}",
            before.current,
            before.maximum,
            before.cached_rate
        );
        assert!((actual_time - expected_time).abs() <= 0.00001);
    }
    assert_eq!(
        resource_snapshot(run, power),
        before,
        "fact reads cannot grant power or change cached/wire values"
    );
}

fn assert_native_cap(run: &mut Run, power: PowerType) {
    for _ in 0..2000 {
        run.app.update();
        let snapshot = resource_snapshot(run, power);
        if snapshot.current == snapshot.maximum {
            assert_eq!(snapshot.cached_rate, 0.0);
            assert_eq!(snapshot.wire.regen_per_sec, 0.0);
            assert_read_only_potential_facts(run, power, read_expected_native_regen(run, power));
            return;
        }
    }
    panic!("native {power:?} did not refill through actual regeneration");
}

fn assert_native_regen_flag(power: PowerType, expected: bool) {
    let path = std::env::var("DPS_PARITY_WORLD_DB").unwrap();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let flags: i64 = db
        .query_row(
            "SELECT Flags FROM power_type WHERE PowerTypeEnum=?",
            [power as i32],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(flags & REGEN_AFFECTED_BY_HASTE != 0, expected);
}

fn native_power(run: &Run, power: PowerType) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, power)
        .unwrap()
        .0
}

fn cast_native_spender(run: &mut Run, spell: u32, power: PowerType) {
    let actor = run.actor;
    let target = if power == PowerType::Mana {
        actor
    } else {
        run.targets[0]
    };
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(crate::spell_cast::CastFailure::OnGlobalCooldown) => run.app.update(),
            Err(error) => panic!("native resource spender{spell}: {error:?}"),
        }
    }
    panic!("native spender GCD never became ready");
}

fn prepare_spent_resource(spec: u32, spell: u32, power: PowerType) -> Run {
    let (profile, traits) = read_reference(spec).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    assert_native_cap(&mut run, power);
    let initial = native_power(&run, power);
    cast_native_spender(&mut run, spell, power);
    for _ in 0..200 {
        if native_power(&run, power) < initial {
            if power != PowerType::Mana {
                assert_eq!(
                    cached_rate(&run, power),
                    0.0,
                    "instant native spend retains the pre-tick zero cache"
                );
            }
            assert_read_only_potential_facts(&run, power, read_expected_native_regen(&run, power));
            return run;
        }
        run.app.update();
    }
    panic!("native spender{spell} did not pay its actual {power:?} cost");
}

fn assert_native_tick(run: &mut Run, power: PowerType, expected_rate: f32) {
    let before = native_power(run, power);
    run.app.update();
    let world = run.app.world();
    let pools = world.get::<PowerPools>(run.actor).unwrap();
    let actual_rate = if power == PowerType::Mana {
        pools.mana_regen_per_sec
    } else {
        pools.get(power).unwrap().regen_per_sec
    };
    let after = native_power(run, power);
    let dt = world.resource::<Time>().delta_secs();
    let maximum = crate::spell_cast::powers::power_value(world, run.actor, power)
        .unwrap()
        .1;
    let rounding = f32::EPSILON * maximum * 8.0;
    println!(
        "NATIVE_REGEN {power:?}: expected={expected_rate} actual={actual_rate} before={before} after={after} dt={dt}"
    );
    assert!((actual_rate - expected_rate).abs() <= rounding);
    assert!((after - before - expected_rate * dt).abs() <= rounding);
    assert_read_only_potential_facts(run, power, expected_rate);
}

fn assert_hasted_native_pool(spec: u32, spell: u32, power: PowerType) {
    assert_native_regen_flag(power, true);
    let mut run = prepare_spent_resource(spec, spell, power);
    let world = run.app.world();
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    assert!(
        stats.haste_pct > 0.0,
        "native reference gear must have haste"
    );
    let expected = read_expected_native_regen(&run, power);
    assert_native_tick(&mut run, power, expected);
    assert_native_cap(&mut run, power);
}

#[test]
#[ignore = "native BM Focus haste; requires current prepared parity inputs"]
fn dps_parity_power_haste_native_bm_focus_scales_actual_regeneration() {
    assert_hasted_native_pool(253, COBRA_SHOT, PowerType::Focus);
}

#[test]
#[ignore = "native WW Energy haste; requires current prepared parity inputs"]
fn dps_parity_power_haste_native_ww_energy_scales_actual_regeneration() {
    assert_hasted_native_pool(269, TIGER_PALM, PowerType::Energy);
}

#[test]
#[ignore = "native unflagged Mana; requires current prepared parity inputs"]
fn dps_parity_power_haste_native_mana_remains_unscaled() {
    assert_native_regen_flag(PowerType::Mana, false);
    let mut run = prepare_spent_resource(66, FLASH_OF_LIGHT, PowerType::Mana);
    let world = run.app.world();
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    assert!(stats.haste_pct > 0.0);
    let expected = read_expected_native_regen(&run, PowerType::Mana);
    assert!(world.get::<Mana>(run.actor).unwrap().current < stats.max_mana);
    assert_native_tick(&mut run, PowerType::Mana, expected);
    assert_native_cap(&mut run, PowerType::Mana);
}

fn rogue_energy(h: &Harness) -> crate::spell_cast::powers::PowerPool {
    *h.app
        .world()
        .get::<PowerPools>(h.caster)
        .unwrap()
        .get(PowerType::Energy)
        .unwrap()
}

fn assert_read_only_energy_facts(h: &Harness, rate: f64, time_to_max: f64) {
    let before = rogue_energy(h);
    let wire = h.app.world().get::<UnitPowers>(h.caster).unwrap().clone();
    let bindings = crate::simc_apl_native::Bindings {
        resources: std::collections::BTreeMap::from([("energy".to_owned(), PowerType::Energy)]),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: h.app.world().resource::<Time>().elapsed_secs_f64(),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let facts =
        crate::simc_apl_native::NativeFacts::new(h.app.world(), h.caster, &bindings, encounter, 1);
    assert_eq!(facts.value("energy.regen").unwrap(), rate);
    assert!((facts.value("energy.time_to_max").unwrap() - time_to_max).abs() < 1e-6);
    assert_eq!(
        rogue_energy(h),
        before,
        "facts cannot grant resource or alter applied rate"
    );
    assert_eq!(*h.app.world().get::<UnitPowers>(h.caster).unwrap(), wire);
}

fn assert_capped_energy(h: &Harness, rate: f64) {
    let energy = rogue_energy(h);
    assert_eq!(
        (energy.current, energy.max, energy.regen_per_sec),
        (100.0, 100.0, 0.0)
    );
    let wire = h
        .app
        .world()
        .get::<UnitPowers>(h.caster)
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Energy)
        .unwrap();
    assert_eq!(
        wire.regen_per_sec, 0.0,
        "cap must preserve zero applied/wire rate"
    );
    assert_read_only_energy_facts(h, rate, 0.0);
}

// Requested 13120 counterexamples: native Energy row and Player class4, with
// explicit0/25% stat inputs. These are unit controls, NOT native-geared parity.
fn assert_rogue_potential_case(haste: f32, rate: f64, time_to_max: f64, after_tick: f32) {
    let mut h = harness(Vec::new(), &[], 4).unwrap();
    h.stop_attack();
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = haste;
    h.advance(1);
    assert_capped_energy(&h, rate);
    crate::spell_cast::powers::change_power(h.app.world_mut(), h.caster, PowerType::Energy, -50.0);
    assert_eq!(
        rogue_energy(&h).regen_per_sec,
        0.0,
        "spend precedes next regen tick"
    );
    assert_read_only_energy_facts(&h, rate, time_to_max);
    h.advance(1);
    assert_eq!(rogue_energy(&h).current, after_tick);
    assert_eq!(f64::from(rogue_energy(&h).regen_per_sec), rate);
    let dt = h.app.world().resource::<Time>().delta_secs_f64();
    for _ in 0..(time_to_max / dt).ceil() as usize + 2 {
        if rogue_energy(&h).current == rogue_energy(&h).max {
            break;
        }
        h.advance(1);
    }
    assert_capped_energy(&h, rate);
}

#[test]
fn dps_parity_power_haste_unflagged_rage_at_center_has_no_phantom_gain() {
    let mut h = harness(Vec::new(), &[], 1).unwrap();
    h.stop_attack();
    h.advance(1);
    let world = h.app.world();
    let info = world
        .resource::<PowerTypeTable>()
        .get(PowerType::Rage)
        .unwrap();
    assert!(!info.regen_affected_by_haste);
    assert!(info.regen_peace < 0.0);
    assert!(
        !world
            .get::<shared::components::CombatStatus>(h.caster)
            .is_some_and(|status| status.0)
    );
    assert_eq!(
        info.regen_interrupt_ms, 0,
        "native Rage uses peace decay immediately"
    );
    let pool = world
        .get::<PowerPools>(h.caster)
        .unwrap()
        .get(PowerType::Rage)
        .unwrap();
    assert_eq!(pool.current, info.center_power as f32);
    assert_eq!(pool.regen_per_sec, 0.0);
    assert_eq!(
        crate::spell_cast::powers::potential_power_regen(world, h.caster, PowerType::Rage),
        Some(0.0)
    );
}

#[test]
fn dps_parity_power_haste_rogue_potential_zero_haste_survives_cap_and_spend() {
    assert_rogue_potential_case(0.0, 10.0, 5.0, 50.5);
}

#[test]
fn dps_parity_power_haste_rogue_potential_twenty_five_haste_survives_cap_and_spend() {
    assert_rogue_potential_case(25.0, 12.5, 4.0, 50.625);
}
