//! Player health and power regeneration on real 50 ms fixed ticks against data/world.db:
//! `Player::RegenerateAll`, `Player::RegenerateHealth`,
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

fn level_10_inputs(in_combat: bool) -> super::powers::HealthRegenInputs {
    super::powers::HealthRegenInputs {
        level: 10,
        max: 1110.0,
        in_combat,
        polymorphed: false,
        standing: true,
        regen_mult: 1.0,
        mod_regen: 0.0,
        regen_during_combat_pct: None,
        regen_in_combat: None,
        base_health_regen: 0.0,
    }
}

#[test]
fn polymorphed_player_regains_a_third_of_max_health_per_tick_even_in_combat() {
    let regen = super::powers::player_health_regen;
    let polymorphed = super::powers::HealthRegenInputs {
        polymorphed: true,
        ..level_10_inputs(true)
    };
    assert_eq!(regen(&polymorphed), 370.0);
    assert_eq!(regen(&level_10_inputs(true)), 0.0);
}

/// Apply every aura effect of `spell_id` from the caster to itself.
fn self_aura(h: &mut Harness, spell_id: u32) {
    let caster = h.caster;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// Health gained over each of `ticks` 2 s regen ticks.
fn tick_gains(h: &mut Harness, ticks: usize) -> Vec<f32> {
    let mut gains = Vec::new();
    let mut last = h.health(h.caster);
    for _ in 0..ticks {
        h.advance(FRAMES_PER_TICK);
        let now = h.health(h.caster);
        gains.push(now - last);
        last = now;
    }
    gains
}

/// Regeneration 20555 (Troll racial): MOD_HEALTH_REGEN_PERCENT 10 makes the level 10
/// out-of-combat tick 22 instead of 20. Its MOD_REGEN_DURING_COMBAT 10 lets health
/// regenerate in combat, but only `Rate.Health` × 10% = 0.1 per tick, and
/// `RegenerateHealth` keeps `m_healthFraction` only from ticks that add a whole point
/// (`if (!integerValue) return;`), so in combat it never adds any.
#[test]
fn troll_regeneration_raises_regen_and_keeps_a_trickle_in_combat() {
    const REGENERATION: u32 = 20_555;
    let Some(mut h) = harness(Vec::new(), &[REGENERATION], MAGE) else {
        return;
    };
    h.advance(1);
    set_combat(&mut h, false);
    set_health(&mut h, 100.0, 1000.0);
    self_aura(&mut h, REGENERATION);
    h.advance(FRAMES_PER_TICK);
    assert_eq!(tick_gains(&mut h, 3), vec![22.0; 3]);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    let inputs = super::powers::HealthRegenInputs {
        regen_mult: mods.health_regen_mult,
        mod_regen: mods.mod_regen,
        regen_during_combat_pct: mods.regen_during_combat_pct,
        regen_in_combat: mods.health_regen_in_combat,
        ..level_10_inputs(true)
    };
    assert!((super::powers::player_health_regen(&inputs) - 0.1).abs() < 1e-6);
    set_combat(&mut h, true);
    assert_eq!(tick_gains(&mut h, 12), vec![0.0; 12]);
}

/// Vitality 20885 (enchant): MOD_HEALTH_REGEN_IN_COMBAT 8 adds 8 health every tick,
/// in combat too.
#[test]
fn vitality_regenerates_eight_health_per_tick_in_combat() {
    const VITALITY: u32 = 20_885;
    let Some(mut h) = harness(Vec::new(), &[VITALITY], MAGE) else {
        return;
    };
    h.advance(1);
    set_health(&mut h, 100.0, 1000.0);
    self_aura(&mut h, VITALITY);
    h.advance(FRAMES_PER_TICK);
    assert_eq!(tick_gains(&mut h, 3), vec![8.0; 3]);
}

/// Food 1127 (Conjured Pumpernickel 1487): MOD_REGEN scaled by SpellScaling, class
/// -2 Consumable at level 10 (1000) × 0.25 = 250 health per 5 s, 100 per tick. It
/// sits the player down, so the level 10 tick is (20 + 100) × 1.5 = 180.
#[test]
fn pumpernickel_mod_regen_heals_its_scaled_amount_seated() {
    const PUMPERNICKEL_FOOD: u32 = 1127;
    let Some(mut h) = harness(Vec::new(), &[PUMPERNICKEL_FOOD], MAGE) else {
        return;
    };
    h.advance(1);
    set_combat(&mut h, false);
    set_health(&mut h, 100.0, 1000.0);
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::UnitLevel(10));
    self_aura(&mut h, PUMPERNICKEL_FOOD);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    assert_eq!(mods.mod_regen, 250.0);
    h.advance(FRAMES_PER_TICK);
    assert_eq!(tick_gains(&mut h, 2), vec![180.0; 2]);
}

/// `m_baseHealthRegen` (gear ITEM_MOD_HEALTH_REGEN 5 per 5 s) adds 2 health every tick
/// and keeps regeneration running in combat (Player.cpp:1626, 1780).
#[test]
fn gear_health_regen_adds_its_share_every_tick_in_combat() {
    let Some(mut h) = harness(Vec::new(), &[], MAGE) else {
        return;
    };
    h.advance(1);
    set_health(&mut h, 100.0, 1000.0);
    let caster = h.caster;
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(caster)
        .unwrap()
        .health_regen = 5.0;
    h.advance(FRAMES_PER_TICK);
    assert_eq!(tick_gains(&mut h, 3), vec![2.0; 3]);
}
