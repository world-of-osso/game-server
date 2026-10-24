//! Native PowerType0x400 resource regeneration, not fabricated haste or pools.
use super::*;
use crate::auras::AuraStatMods;
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};
use crate::spell_cast::test_harness::{Harness, harness};
use server::simc_apl::Facts;
use shared::components::{Mana, PowerType, UnitPowers};

const REGEN_AFFECTED_BY_HASTE: i64 = 0x0400;
const COBRA_SHOT: u32 = 193455;
const TIGER_PALM: u32 = 100780;
const FLASH_OF_LIGHT: u32 = 19750;

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
    let initial = native_power(&run, power);
    cast_native_spender(&mut run, spell, power);
    for _ in 0..200 {
        if native_power(&run, power) < initial {
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
    let info = world.resource::<PowerTypeTable>().get(power).unwrap();
    let mods = world.get::<AuraStatMods>(run.actor).unwrap();
    let multiplier = mods.power_regen(power as i32);
    let flat = mods.power_regen_flat(power as i32) / 5.0;
    let expected = info.regen_combat * multiplier * (1.0 + stats.haste_pct / 100.0) + flat;
    println!(
        "NATIVE_REGEN_INPUT spec={spec} haste_pct={} base={} multiplier={multiplier} flat={flat}",
        stats.haste_pct, info.regen_combat
    );
    assert_native_tick(&mut run, power, expected);
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
    let mods = world.get::<AuraStatMods>(run.actor).unwrap();
    let expected = stats.base_mana * 0.01 * mods.power_regen(PowerType::Mana as i32)
        + mods.power_regen_flat(PowerType::Mana as i32) / 5.0;
    assert!(world.get::<Mana>(run.actor).unwrap().current < stats.max_mana);
    assert_native_tick(&mut run, PowerType::Mana, expected);
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
fn dps_parity_power_haste_rogue_potential_zero_haste_survives_cap_and_spend() {
    assert_rogue_potential_case(0.0, 10.0, 5.0, 50.5);
}

#[test]
fn dps_parity_power_haste_rogue_potential_twenty_five_haste_survives_cap_and_spend() {
    assert_rogue_potential_case(25.0, 12.5, 4.0, 50.625);
}
