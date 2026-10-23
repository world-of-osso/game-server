//! Native PowerType0x400 resource regeneration, not fabricated haste or pools.
use super::*;
use crate::auras::AuraStatMods;
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};
use shared::components::{Mana, PowerType};

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
