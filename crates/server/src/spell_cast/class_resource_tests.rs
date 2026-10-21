//! Class secondary resources against data/world.db:
//! power maxima, druid forms, runes, charged combo points, partial power.

use shared::components::{PowerType, UnitPowers};

use super::test_harness::*;

const PALADIN: u8 = 2;
const MONK: u8 = 10;
/// Boundless Conviction: MOD_MAX_POWER 418, Holy Power +2.
const BOUNDLESS_CONVICTION: u32 = 115675;
/// Windwalker Monk spec passive: MOD_INCREASE_ENERGY 35, Chi +1.
const WINDWALKER_PASSIVE: u32 = 1258122;
/// Ascension talent: MOD_INCREASE_ENERGY 35, Chi +1.
const ASCENSION: u32 = 115396;
/// Windwalker `chr_classes_x_power_types` (Monk 10) without the alternate bars.
const MONK_POOLS: Option<&[PowerType]> =
    Some(&[PowerType::Energy, PowerType::Mana, PowerType::Chi]);

fn replicated(h: &Harness, power: PowerType) -> (i32, i32) {
    let powers = h.app.world().get::<UnitPowers>(h.caster).unwrap();
    let entry = powers
        .entries
        .iter()
        .find(|entry| entry.power == power)
        .unwrap_or_else(|| panic!("{power:?} in {powers:?}"));
    (entry.current, entry.max)
}

/// Replicated max of `power` for a `class` caster knowing `passives`, with the class
/// pools `pools` when given (Monk is not in `ClassSpells`: no playable Monk yet).
fn replicated_max(
    class: u8,
    passives: &[u32],
    power: PowerType,
    pools: Option<&[PowerType]>,
) -> Option<i32> {
    let mut h = harness(Vec::new(), passives, class)?;
    if let Some(order) = pools {
        let table = power_table(world_db()?);
        let caster = h.caster;
        h.app
            .world_mut()
            .entity_mut(caster)
            .insert(super::powers::PowerPools::from_table(
                &table,
                order.to_vec(),
            ));
    }
    h.advance(2);
    Some(replicated(&h, power).1)
}

#[test]
fn holy_power_max_is_three_base_and_five_with_boundless_conviction() {
    let Some(base) = replicated_max(PALADIN, &[], PowerType::HolyPower, None) else {
        return;
    };
    assert_eq!(base, 3);
    let with =
        replicated_max(PALADIN, &[BOUNDLESS_CONVICTION], PowerType::HolyPower, None).unwrap();
    assert_eq!(with, 5);
}

#[test]
fn chi_max_is_four_base_five_for_windwalker_and_six_with_ascension() {
    let Some(base) = replicated_max(MONK, &[], PowerType::Chi, MONK_POOLS) else {
        return;
    };
    assert_eq!(base, 4);
    assert_eq!(
        replicated_max(MONK, &[WINDWALKER_PASSIVE], PowerType::Chi, MONK_POOLS).unwrap(),
        5
    );
    assert_eq!(
        replicated_max(
            MONK,
            &[WINDWALKER_PASSIVE, ASCENSION],
            PowerType::Chi,
            MONK_POOLS
        )
        .unwrap(),
        6
    );
}

#[test]
fn losing_a_max_power_passive_clamps_current_holy_power() {
    let Some(mut h) = harness(Vec::new(), &[BOUNDLESS_CONVICTION], PALADIN) else {
        return;
    };
    h.advance(2);
    h.set_power(PowerType::HolyPower, 5.0);
    h.advance(1);
    assert_eq!(replicated(&h, PowerType::HolyPower), (5, 5));
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    // Update queues the passive sync, the next tick removes the aura, the one after
    // applies the max.
    h.advance(3);
    assert_eq!(replicated(&h, PowerType::HolyPower), (3, 3));
}

const DRUID: u8 = 11;
const CAT_FORM: u32 = 768;
const BEAR_FORM: u32 = 5487;
/// Human male native display.
const NATIVE_DISPLAY: u32 = 49;

fn primary_power(h: &Harness) -> PowerType {
    h.app.world().get::<UnitPowers>(h.caster).unwrap().entries[0].power
}

fn display(h: &Harness) -> u32 {
    h.app
        .world()
        .get::<shared::components::ModelDisplay>(h.caster)
        .unwrap()
        .display_id
}

#[test]
fn druid_forms_switch_display_power_and_model_and_exclude_each_other() {
    let Some(mut h) = harness(Vec::new(), &[CAT_FORM, BEAR_FORM], DRUID) else {
        return;
    };
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::ModelDisplay {
            display_id: NATIVE_DISPLAY,
        });
    h.advance(1);
    assert_eq!(primary_power(&h), PowerType::Mana);

    // Cat Form: SpellShapeshiftForm 1 (CreatureDisplayID 115603), energy
    // (`CalculateDisplayPowerType`), combo points still listed.
    h.cast_and_deliver(CAT_FORM, None);
    h.advance(1);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(primary_power(&h), PowerType::Energy);
    assert_eq!(display(&h), 115603);
    let powers: Vec<PowerType> = h
        .app
        .world()
        .get::<UnitPowers>(caster)
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.power)
        .collect();
    assert_eq!(
        powers,
        [
            PowerType::Energy,
            PowerType::Mana,
            PowerType::Rage,
            PowerType::ComboPoints,
            PowerType::LunarPower
        ]
    );

    // Bear Form replaces Cat Form: form 5 (display 115602), rage.
    h.advance(40);
    h.cast_and_deliver(BEAR_FORM, None);
    h.advance(1);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(!crate::auras::has_aura(h.app.world(), caster, CAT_FORM));
    assert_eq!(primary_power(&h), PowerType::Rage);
    assert_eq!(display(&h), 115602);

    // Leaving the form restores mana and the native model.
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BEAR_FORM,
    );
    h.advance(1);
    assert_eq!(primary_power(&h), PowerType::Mana);
    assert_eq!(display(&h), NATIVE_DISPLAY);
}

const DEATH_KNIGHT: u8 = 6;
/// Obliterate: SpellPower 2 runes (PowerType 5).
const OBLITERATE: u32 = 49020;

fn runes(h: &Harness) -> super::runes::Runes {
    h.app
        .world()
        .get::<super::runes::Runes>(h.caster)
        .unwrap()
        .clone()
}

fn spend_runes(h: &mut Harness, count: f32) {
    let caster = h.caster;
    super::powers::change_power(h.app.world_mut(), caster, PowerType::Runes, -count);
}

#[test]
fn obliterate_puts_the_two_lowest_ready_runes_on_cooldown() {
    let Some(mut h) =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[OBLITERATE], DEATH_KNIGHT)
    else {
        return;
    };
    h.advance(1);
    assert_eq!(replicated(&h, PowerType::Runes), (6, 6));
    h.cast_and_deliver(OBLITERATE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let state = runes(&h);
    assert_eq!(state.order, [0, 1]);
    assert!(
        state.cooldowns[0] > 9.8 && state.cooldowns[1] > 9.8,
        "{state:?}"
    );
    assert!(state.cooldowns[2..].iter().all(|&secs| secs == 0.0));
    assert_eq!(replicated(&h, PowerType::Runes), (4, 6));
}

#[test]
fn only_three_runes_recharge_at_a_time_in_depletion_order() {
    let Some(mut h) = harness(Vec::new(), &[], DEATH_KNIGHT) else {
        return;
    };
    h.advance(1);
    spend_runes(&mut h, 4.0);
    // 2 s later the last two go.
    h.advance(40);
    spend_runes(&mut h, 2.0);
    assert_eq!(runes(&h).order, [0, 1, 2, 3, 4, 5]);
    // Runes 0-2 started at t0; rune 3 waits for a free slot.
    h.advance(1);
    let state = runes(&h);
    assert!((7.8..8.0).contains(&state.cooldowns[0]), "{state:?}");
    assert_eq!(state.cooldowns[3], 10.0);
    assert_eq!(state.cooldowns[4], 10.0);

    // t0 + 10 s: runes 0-2 are ready, 3-5 recharge.
    h.advance(162);
    let state = runes(&h);
    assert_eq!(state.order, [3, 4, 5], "{state:?}");
    assert_eq!(replicated(&h, PowerType::Runes), (3, 6));
    assert!(
        state.cooldowns[3..].iter().all(|&secs| secs > 9.5),
        "{state:?}"
    );

    // t0 + 20 s: all six ready.
    h.advance(201);
    assert!(runes(&h).order.is_empty(), "{:?}", runes(&h));
    assert_eq!(replicated(&h, PowerType::Runes), (6, 6));
}

#[test]
fn waiting_runes_are_ready_one_full_cooldown_after_a_slot_frees() {
    let mut state = super::runes::Runes::new(6);
    assert_eq!(state.spend(6, 10.0), 6);
    // Runes 0-2 recharge; after 4 s they have 6 s left and 3-5 wait for them.
    for _ in 0..80 {
        state.regenerate(0.05);
    }
    let ready_in = state.ready_in(10.0);
    let expected = [6.0, 6.0, 6.0, 16.0, 16.0, 16.0];
    for (actual, expected) in ready_in.iter().zip(expected) {
        assert!((actual - expected).abs() < 0.01, "{ready_in:?}");
    }
    // Spending a ready rune is impossible while all six are depleted.
    assert_eq!(state.spend(1, 10.0), 0);
}

const ROGUE: u8 = 4;
/// Echoing Reprimand 354835: SET_POWER_POINT_CHARGE 494 on points 2, 3, 4 and 5.
const ECHOING_REPRIMAND_ALL: u32 = 354835;
/// Supercharge 470398 / 470414: SET_POWER_POINT_CHARGE on point 1 / 5.
const SUPERCHARGE_1: u32 = 470398;
const SUPERCHARGE_5: u32 = 470414;

fn apply_self_aura(h: &mut Harness, spell_id: u32) {
    let caster = h.caster;
    let world = h.app.world_mut();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn charged_points(h: &Harness) -> Vec<u8> {
    h.app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap()
        .charged_power_points
        .clone()
}

#[test]
fn set_power_point_charge_auras_charge_their_combo_point_indexes() {
    let Some(mut h) = harness(
        Vec::new(),
        &[ECHOING_REPRIMAND_ALL, SUPERCHARGE_1, SUPERCHARGE_5],
        ROGUE,
    ) else {
        return;
    };
    h.advance(1);
    apply_self_aura(&mut h, SUPERCHARGE_5);
    apply_self_aura(&mut h, SUPERCHARGE_1);
    assert_eq!(charged_points(&h), [1, 5]);
    h.advance(1);
    assert_eq!(
        h.app
            .world()
            .get::<UnitPowers>(h.caster)
            .unwrap()
            .charged_points,
        [1, 5]
    );
    apply_self_aura(&mut h, ECHOING_REPRIMAND_ALL);
    assert_eq!(charged_points(&h), [1, 2, 3, 4, 5]);
    h.advance(1);
    assert_eq!(
        h.app
            .world()
            .get::<UnitPowers>(h.caster)
            .unwrap()
            .charged_points,
        [1, 2, 3, 4, 5]
    );
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id != ECHOING_REPRIMAND_ALL,
    );
    assert_eq!(charged_points(&h), [2, 3, 4, 5]);
    h.advance(1);
    let powers = h.app.world().get::<UnitPowers>(caster).unwrap();
    assert_eq!(powers.charged_points, [2, 3, 4, 5]);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Consumed,
        |_| true,
    );
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<UnitPowers>(caster)
            .unwrap()
            .charged_points
            .is_empty()
    );
}

fn runes_view(h: &Harness) -> shared::components::UnitRunes {
    h.app
        .world()
        .get::<shared::components::UnitRunes>(h.caster)
        .unwrap()
        .clone()
}

#[test]
fn unit_runes_replicate_ready_times_with_waiting_runes_starting_later() {
    let Some(mut h) = harness(Vec::new(), &[], DEATH_KNIGHT) else {
        return;
    };
    h.advance(1);
    assert_eq!(
        runes_view(&h),
        shared::components::UnitRunes {
            duration_ms: 10_000,
            ready_in_ms: vec![0; 6],
        }
    );
    spend_runes(&mut h, 5.0);
    h.advance(1);
    let view = runes_view(&h);
    assert_eq!(view.duration_ms, 10_000);
    // Runes 0-2 recharge (one tick in); 3 and 4 start when 0 and 1 finish.
    for (index, expected) in [9_950, 9_950, 9_950, 19_950, 19_950, 0]
        .into_iter()
        .enumerate()
    {
        let actual = view.ready_in_ms[index];
        assert!(actual.abs_diff(expected) <= 20, "rune {index}: {view:?}");
    }
    // No resend while the order holds: the client counts down.
    h.advance(20);
    assert_eq!(runes_view(&h), view);
    h.advance(180);
    let completed = runes_view(&h);
    assert_eq!(&completed.ready_in_ms[..3], &[0; 3]);
    assert_eq!(completed.ready_in_ms[5], 0);
    for &remaining in &completed.ready_in_ms[3..5] {
        assert!((9_900..=10_000).contains(&remaining), "{completed:?}");
    }
    h.advance(201);
    assert_eq!(runes_view(&h).ready_in_ms, [0; 6]);
}

#[test]
fn rune_replication_sends_haste_scaled_duration_and_resends_rate_changes() {
    let mut h = harness(Vec::new(), &[], DEATH_KNIGHT).expect("resource fixture requires world.db");
    h.advance(1);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 100.0;
    spend_runes(&mut h, 5.0);
    h.advance(1);
    let view = runes_view(&h);
    assert_eq!(view.duration_ms, 5_000);
    for (actual, expected) in view
        .ready_in_ms
        .iter()
        .zip([4_950, 4_950, 4_950, 9_950, 9_950, 0])
    {
        assert!(actual.abs_diff(expected) <= 20, "{view:?}");
    }
    h.advance(20);
    assert_eq!(runes_view(&h), view);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .haste_pct = 0.0;
    h.advance(1);
    let view = runes_view(&h);
    assert_eq!(view.duration_ms, 10_000);
    for (actual, expected) in view
        .ready_in_ms
        .iter()
        .zip([3_900, 3_900, 3_900, 13_900, 13_900, 0])
    {
        assert!(actual.abs_diff(expected) <= 20, "{view:?}");
    }
}

fn power_entry(h: &Harness, power: PowerType) -> shared::components::PowerEntry {
    h.app
        .world()
        .get::<UnitPowers>(h.caster)
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.power == power)
        .unwrap()
        .clone()
}

#[test]
fn replication_energy_stops_predicting_on_the_tick_it_reaches_max() {
    let mut h = harness(Vec::new(), &[], ROGUE).expect("resource fixture requires world.db");
    h.advance(1);
    h.set_power(PowerType::Energy, 99.75);
    h.advance(1);
    let entry = power_entry(&h, PowerType::Energy);
    assert_eq!(h.power(PowerType::Energy), 100.0);
    assert_eq!(
        (entry.current, entry.partial, entry.regen_per_sec),
        (100, 0, 0.0)
    );
}

#[test]
fn replication_rage_stops_predicting_on_the_tick_it_reaches_zero() {
    let mut h = harness(Vec::new(), &[], 1).expect("resource fixture requires world.db");
    h.advance(1);
    h.set_power(PowerType::Rage, 0.25);
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.advance(1);
    let entry = power_entry(&h, PowerType::Rage);
    assert_eq!(h.power(PowerType::Rage), 0.0);
    assert_eq!(
        (entry.current, entry.partial, entry.regen_per_sec),
        (0, 0, 0.0)
    );
}

#[test]
fn replication_mana_stops_predicting_on_the_tick_it_reaches_max() {
    let mut h = harness(Vec::new(), &[], PALADIN).expect("resource fixture requires world.db");
    h.advance(1);
    h.app
        .world_mut()
        .get_mut::<shared::components::Mana>(h.caster)
        .unwrap()
        .current = 9_999.0;
    h.advance(1);
    let entry = power_entry(&h, PowerType::Mana);
    assert_eq!(
        (entry.current, entry.partial, entry.regen_per_sec),
        (10_000, 0, 0.0)
    );
}

#[test]
fn replication_rune_count_tracks_a_max_power_passive_without_a_spend() {
    const RUNIC_LONGEVITY: u32 = 194653;
    // The real passive also adds rune regen (aura 110), changing duration and
    // masking a missing rune-count send. Isolate its +1 max-power effect.
    let mut passive = world_db()
        .expect("resource fixture requires world.db")
        .store
        .get(RUNIC_LONGEVITY)
        .unwrap()
        .clone();
    passive.effects.retain(|effect| effect.aura == 418);
    let mut h =
        harness(vec![passive], &[], DEATH_KNIGHT).expect("resource fixture requires world.db");
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    assert_eq!(runes_view(&h).ready_in_ms, [0; 6]);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            RUNIC_LONGEVITY,
        ]));
    h.advance(3);
    assert_eq!(replicated(&h, PowerType::Runes), (7, 7));
    assert_eq!(runes_view(&h).ready_in_ms, [0; 7]);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    assert_eq!(replicated(&h, PowerType::Runes), (6, 6));
    assert_eq!(runes_view(&h).ready_in_ms, [0; 6]);
}

#[test]
fn replication_forms_send_energy_regen_and_combat_dependent_rage_decay() {
    let mut h = harness(Vec::new(), &[CAT_FORM, BEAR_FORM], DRUID)
        .expect("resource fixture requires world.db");
    h.cast_and_deliver(CAT_FORM, None);
    h.set_power(PowerType::Energy, 20.25);
    h.advance(1);
    let entry = power_entry(&h, PowerType::Energy);
    assert_eq!(primary_power(&h), PowerType::Energy);
    assert_eq!(
        (entry.current, entry.partial, entry.regen_per_sec),
        (20, 750, 10.0)
    );

    h.advance(40);
    h.cast_and_deliver(BEAR_FORM, None);
    h.set_power(PowerType::Rage, 100.0);
    h.advance(1);
    assert_eq!(primary_power(&h), PowerType::Rage);
    assert_eq!(power_entry(&h, PowerType::Rage).regen_per_sec, 0.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.advance(1);
    let entry = power_entry(&h, PowerType::Rage);
    assert_eq!(
        (entry.current, entry.partial, entry.regen_per_sec),
        (99, 375, -12.5)
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = true;
    h.advance(1);
    assert_eq!(power_entry(&h, PowerType::Rage).regen_per_sec, 0.0);
}

/// TC StatSystem.cpp931: aura94 selects combat regeneration without entering combat.
#[test]
fn census_tail_94_native_bear_form_retains_idle_rage_until_removed() {
    const BEAR_RAGE: u32 = 17_057;
    let mut h = harness(Vec::new(), &[BEAR_RAGE], DRUID).unwrap();
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.advance(80);
    h.cast_and_deliver(BEAR_RAGE, None);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, BEAR_RAGE));
    assert!(
        !h.app
            .world()
            .get::<shared::components::CombatStatus>(h.caster)
            .unwrap()
            .0
    );
    let before = h.power(PowerType::Rage);
    assert!(before > 0.0, "native effect0 must grant Rage");
    h.advance(20);
    assert_eq!(h.power(PowerType::Rage), before);
    assert_eq!(power_entry(&h, PowerType::Rage).regen_per_sec, 0.0);
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BEAR_RAGE,
    );
    h.advance(20);
    assert!(
        h.power(PowerType::Rage) < before,
        "removal restores native idle decay"
    );
    assert_eq!(power_entry(&h, PowerType::Rage).regen_per_sec, -12.5);
}

const EVOKER: u8 = 13;
/// Evoker `chr_classes_x_power_types` without the alternate bars (Evoker is not in
/// `ClassSpells` yet).
const EVOKER_POOLS: &[PowerType] = &[PowerType::Mana, PowerType::Essence];

#[test]
fn essence_regenerates_a_fifth_per_second_and_replicates_the_partial_and_rate() {
    let Some(mut h) = harness(Vec::new(), &[], EVOKER) else {
        return;
    };
    let table = power_table(world_db().unwrap());
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(super::powers::PowerPools::from_table(
            &table,
            EVOKER_POOLS.to_vec(),
        ));
    h.advance(1);
    h.set_power(PowerType::Essence, 2.0);
    h.advance(1);
    let first = power_entry(&h, PowerType::Essence);
    assert_eq!(first.current, 2);
    assert!((9..=10).contains(&first.partial), "{first:?}");
    assert!((first.regen_per_sec - 0.2).abs() < 1e-6, "{first:?}");
    // 3 s at `RegenCombat` 0.2/s: 2.6 essence, without a partial-only resend.
    h.advance(59);
    let essence = h.power(PowerType::Essence);
    assert!((essence - 2.6).abs() < 0.02, "{essence}");
    let powers = h.app.world().get::<UnitPowers>(caster).unwrap().clone();
    let entry = powers
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Essence)
        .unwrap();
    assert_eq!(entry.current, 2);
    assert!((entry.regen_per_sec - 0.2).abs() < 1e-6, "{entry:?}");
    // The first send includes the regen tick; fraction changes alone do not resend.
    assert_eq!(*entry, first);

    // Crossing 3.0 resends with the fraction at that tick (3.0 at +2 s).
    h.advance(42);
    let powers = h.app.world().get::<UnitPowers>(caster).unwrap();
    let entry = powers
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Essence)
        .unwrap();
    assert_eq!(entry.current, 3);
    assert!(entry.partial < 60, "{entry:?}");

    // Full: the rate drops to 0 and is resent.
    h.set_power(PowerType::Essence, 5.0);
    h.advance(1);
    let powers = h.app.world().get::<UnitPowers>(caster).unwrap();
    let entry = powers
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Essence)
        .unwrap();
    assert_eq!((entry.current, entry.max, entry.regen_per_sec), (5, 5, 0.0));
}

/// A player spawns without `ModelDisplay` (its native model is its race and sex). Cat
/// Form still shows its display (Unit::SetDisplayId, 115603), and cancelling it gives the
/// native model back (RestoreDisplayId): `ModelDisplay` goes away.
#[test]
fn a_player_without_model_display_shows_cat_form_and_gets_its_native_model_back() {
    let Some(mut h) = harness(Vec::new(), &[CAT_FORM], DRUID) else {
        return;
    };
    let caster = h.caster;
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(caster)
            .is_none()
    );
    h.cast_and_deliver(CAT_FORM, None);
    h.advance(1);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(display(&h), 115603);
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == CAT_FORM,
    );
    h.advance(1);
    assert!(!crate::auras::has_aura(h.app.world(), caster, CAT_FORM));
    assert!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(caster)
            .is_none()
    );
}
