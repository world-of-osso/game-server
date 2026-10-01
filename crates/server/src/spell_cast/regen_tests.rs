//! Player health and power regeneration on real 50 ms fixed ticks against data/world.db
//! (skipped when absent): `Player::RegenerateAll`, `Player::RegenerateHealth`,
//! `Player::Regenerate`.

use shared::components::{CombatStatus, Health, Level, Mana, PowerType, UnitPowers};

use super::test_harness::*;

const MAGE: u8 = 8;
const HUNTER: u8 = 3;
const DEATH_KNIGHT: u8 = 6;
/// 2 s of 50 ms frames.
const FRAMES_PER_TICK: usize = 40;

fn set_health(h: &mut Harness, current: f32, max: f32) {
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(Health { current, max });
}

fn set_combat(h: &mut Harness, in_combat: bool) {
    h.app
        .world_mut()
        .get_mut::<CombatStatus>(h.caster)
        .unwrap()
        .0 = in_combat;
}

/// `(frame, health)` at every frame of `frames` where the caster's health changed.
fn health_changes(h: &mut Harness, frames: usize) -> Vec<(usize, f32)> {
    let mut last = h.health(h.caster);
    let mut changes = Vec::new();
    for frame in 0..frames {
        h.advance(1);
        let health = h.health(h.caster);
        if health != last {
            changes.push((frame, health));
            last = health;
        }
    }
    changes
}

fn gains_and_gaps(changes: &[(usize, f32)], start: f32) -> (Vec<f32>, Vec<usize>) {
    let mut previous = start;
    let gains = changes
        .iter()
        .map(|&(_, health)| {
            let gain = health - previous;
            previous = health;
            gain
        })
        .collect();
    let gaps = changes
        .windows(2)
        .map(|pair| pair[1].0 - pair[0].0)
        .collect();
    (gains, gaps)
}

fn mage_out_of_combat(current: f32, max: f32) -> Option<Harness> {
    let mut h = harness(Vec::new(), &[], MAGE)?;
    h.advance(1);
    set_combat(&mut h, false);
    set_health(&mut h, current, max);
    Some(h)
}

#[test]
fn level_10_player_out_of_combat_regains_a_fiftieth_of_max_health_every_2_seconds() {
    let Some(mut h) = mage_out_of_combat(600.0, 1000.0) else {
        return;
    };
    // 20% of max / level 10 = 20 health per 2 s tick.
    let changes = health_changes(&mut h, 5 * FRAMES_PER_TICK);
    let (gains, gaps) = gains_and_gaps(&changes, 600.0);
    assert_eq!(gains, vec![20.0; 5], "{changes:?}");
    assert_eq!(gaps, vec![FRAMES_PER_TICK; 4], "{changes:?}");
    assert_eq!(h.health(h.caster), 700.0);
}

#[test]
fn level_20_player_regains_one_and_a_half_percent_of_max_health_per_tick() {
    let Some(mut h) = mage_out_of_combat(600.0, 1000.0) else {
        return;
    };
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(Level(20));
    let changes = health_changes(&mut h, 3 * FRAMES_PER_TICK);
    let (gains, _) = gains_and_gaps(&changes, 600.0);
    assert_eq!(gains, vec![15.0; 3], "{changes:?}");
}

#[test]
fn player_in_combat_does_not_regenerate_health_until_combat_ends() {
    let Some(mut h) = mage_out_of_combat(600.0, 1000.0) else {
        return;
    };
    set_combat(&mut h, true);
    assert_eq!(health_changes(&mut h, 5 * FRAMES_PER_TICK), vec![]);
    set_combat(&mut h, false);
    let changes = health_changes(&mut h, FRAMES_PER_TICK);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].1, 620.0);
}

/// The observed level 10 mage: 666/1110 and 1109/1110 used to stay put. 22.2 health per
/// tick (whole 22, fraction carried) reaches exactly 1110, never 1109.
#[test]
fn damaged_level_10_mage_climbs_to_exactly_full_health_and_replicates_it() {
    let Some(mut h) = mage_out_of_combat(666.0, 1110.0) else {
        return;
    };
    // 444 missing / 22.2 per tick = 20 ticks.
    let changes = health_changes(&mut h, 21 * FRAMES_PER_TICK);
    let (gains, _) = gains_and_gaps(&changes, 666.0);
    assert_eq!(changes.len(), 20, "{changes:?}");
    assert!(
        gains.iter().all(|&gain| gain == 22.0 || gain == 23.0),
        "{gains:?}"
    );
    assert_eq!(h.health(h.caster), 1110.0);
    assert!(health_changes(&mut h, 2 * FRAMES_PER_TICK).is_empty());

    set_health(&mut h, 1109.0, 1110.0);
    let changes = health_changes(&mut h, FRAMES_PER_TICK);
    assert_eq!(
        changes
            .iter()
            .map(|&(_, health)| health)
            .collect::<Vec<_>>(),
        [1110.0]
    );
}

#[test]
fn dead_player_regenerates_neither_health_nor_mana() {
    let Some(mut h) = mage_out_of_combat(600.0, 1000.0) else {
        return;
    };
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Ghost,
            ..Default::default()
        });
    h.app.world_mut().get_mut::<Mana>(caster).unwrap().current = 0.0;
    assert_eq!(health_changes(&mut h, 3 * FRAMES_PER_TICK), vec![]);
    assert_eq!(h.app.world().get::<Mana>(caster).unwrap().current, 0.0);
}

fn mana(h: &Harness) -> f32 {
    h.app.world().get::<Mana>(h.caster).unwrap().current
}

/// Mana `PowerType` has no `UseRegenInterrupt` flag and `RegenPeace` = `RegenCombat` = 0,
/// so `Player::GetPowerRegen` gives base mana / 100 per second in and out of combat
/// (StatSystem.cpp:874-883, 926-935): spending mana starts no five-second pause.
#[test]
fn mana_regenerates_one_percent_of_base_mana_per_second_in_and_out_of_combat() {
    let Some(mut h) = harness(Vec::new(), &[], MAGE) else {
        return;
    };
    h.advance(1);
    for in_combat in [true, false] {
        set_combat(&mut h, in_combat);
        h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 0.0;
        // 10 000 base mana: 100 per second.
        h.advance(20);
        assert!(
            (mana(&h) - 100.0).abs() < 0.1,
            "combat {in_combat}: {}",
            mana(&h)
        );
        h.advance(20);
        assert!(
            (mana(&h) - 200.0).abs() < 0.1,
            "combat {in_combat}: {}",
            mana(&h)
        );
    }
    let entry = h
        .app
        .world()
        .get::<UnitPowers>(h.caster)
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Mana)
        .cloned()
        .unwrap();
    assert_eq!(
        (entry.current, entry.max, entry.regen_per_sec),
        (200, 10_000, 100.0)
    );
}

/// Focus `RegenPeace` = `RegenCombat` = 5 per second.
#[test]
fn hunter_focus_regenerates_five_per_second_in_combat() {
    let Some(mut h) = harness(Vec::new(), &[], HUNTER) else {
        return;
    };
    h.advance(1);
    h.set_power(PowerType::Focus, 0.0);
    h.advance(40);
    assert!(
        (h.power(PowerType::Focus) - 10.0).abs() < 0.01,
        "{}",
        h.power(PowerType::Focus)
    );
}

/// Runic Power (tenths): `RegenCombat` 0 holds it in combat, `RegenPeace` -12.5 decays
/// 1.25 runic power per second out of combat.
#[test]
fn runic_power_holds_in_combat_and_decays_out_of_combat() {
    let Some(mut h) = harness(Vec::new(), &[], DEATH_KNIGHT) else {
        return;
    };
    h.advance(1);
    h.set_power(PowerType::RunicPower, 500.0);
    h.advance(40);
    assert_eq!(h.power(PowerType::RunicPower), 500.0);
    set_combat(&mut h, false);
    h.advance(40);
    let runic = h.power(PowerType::RunicPower);
    assert!((runic - 475.0).abs() < 0.01, "{runic}");
}

#[test]
fn polymorphed_player_regains_a_third_of_max_health_per_tick_even_in_combat() {
    assert_eq!(
        super::powers::player_health_regen(10, 1110.0, true, true),
        370.0
    );
    assert_eq!(
        super::powers::player_health_regen(10, 1110.0, true, false),
        0.0
    );
}
