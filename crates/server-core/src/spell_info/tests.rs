//! Real-DB tests against the published data/world.db. Each test skips
//! when the DB is absent.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

use super::*;
use rusqlite::{Connection, OpenFlags};

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
}

fn open_world_db() -> Option<Connection> {
    let path = world_db_path();
    Some(Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap())
}

fn resident_kib() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|value| value.trim().trim_end_matches("kB").trim().parse().ok())
        .unwrap_or(0)
}

/// Loaded once per test binary; prints load time and RSS growth.
fn world() -> Option<&'static (SpellStore, ClassSpells)> {
    static WORLD: OnceLock<Option<(SpellStore, ClassSpells)>> = OnceLock::new();
    WORLD
        .get_or_init(|| {
            let conn = open_world_db()?;
            let rss_before = resident_kib();
            let started = Instant::now();
            let loaded = load_spell_data(&conn).expect("load spell data");
            eprintln!(
                "spell data: {} spells in {:?}, RSS +{} MiB (SpellInfo {} B, SpellEffect {} B)",
                loaded.0.len(),
                started.elapsed(),
                resident_kib().saturating_sub(rss_before) / 1024,
                size_of::<SpellInfo>(),
                size_of::<SpellEffect>()
            );
            Some(loaded)
        })
        .as_ref()
}

fn spell(id: u32) -> Option<&'static SpellInfo> {
    let (store, _) = world()?;
    Some(
        store
            .get(id)
            .unwrap_or_else(|| panic!("spell {id} missing")),
    )
}

fn class(class_id: u8) -> Option<&'static ClassInfo> {
    let (_, classes) = world()?;
    Some(
        classes
            .classes
            .iter()
            .find(|class| class.class_id == class_id)
            .unwrap_or_else(|| panic!("class {class_id} missing")),
    )
}

#[test]
fn store_indexes_every_spell_name_row() {
    let (Some(conn), Some((store, classes))) = (open_world_db(), world()) else {
        return;
    };
    let mut names = conn
        .prepare("SELECT ID, Name_lang FROM spell_name")
        .unwrap();
    let names = names
        .query_map([], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap();
    for row in names {
        let (id, name) = row.unwrap();
        let loaded = store
            .get(id)
            .unwrap_or_else(|| panic!("spell {id} missing"));
        assert_eq!(loaded.name, name, "spell {id}");
    }
    // TC also loads non-conflicting DifficultyID0 server-side spells; those
    // additional entries must not make indexing every file spell a failure.
    let class_ids: Vec<u8> = classes.classes.iter().map(|c| c.class_id).collect();
    assert_eq!(class_ids, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
}

#[test]
fn fireball_has_percent_mana_cost_and_cast_time() {
    let Some(fireball) = spell(133) else { return };
    assert_eq!(fireball.name, "Fireball");
    let misc = fireball.misc.as_ref().unwrap();
    assert_eq!(misc.school_mask, 4);
    assert_eq!(misc.cast_time.unwrap().base_ms, 1750);
    assert_eq!(misc.range.unwrap().max_hostile, 40.0);
    let cost = &fireball.power_costs[0];
    assert_eq!((cost.power_type, cost.mana_cost), (0, 0));
    assert_eq!(cost.power_cost_pct, 1.0);
}

#[test]
fn kick_interrupts_with_effect_68_and_has_a_cooldown() {
    let Some(kick) = spell(1766) else { return };
    let effect = &kick.effects[0];
    assert_eq!((effect.index, effect.effect, effect.mechanic), (0, 68, 26));
    assert_eq!(effect.implicit_targets, [6, 0]);
    assert_eq!(kick.cooldowns.as_ref().unwrap().recovery_ms, 15_000);
    let categories = kick.categories.as_ref().unwrap();
    assert_eq!(
        (categories.defense_type, categories.prevention_type),
        (2, 2)
    );
    let duration = kick.misc.as_ref().unwrap().duration.unwrap();
    assert_eq!(duration.duration_ms, 6000, "interrupt lockout length");
}

#[test]
fn purify_dispels_with_effect_38_keeping_both_dispel_types() {
    let Some(purify) = spell(527) else { return };
    let dispels: Vec<(u32, i32)> = purify
        .effects
        .iter()
        .map(|effect| (effect.effect, effect.misc_values[0]))
        .collect();
    assert_eq!(dispels, [(38, 1), (38, 3)]);
}

#[test]
fn death_strike_costs_runic_power_type_6_in_tenths() {
    let Some(death_strike) = spell(49998) else {
        return;
    };
    let cost = &death_strike.power_costs[0];
    assert_eq!((cost.power_type, cost.mana_cost), (6, 450));
}

#[test]
fn chaos_bolt_costs_soul_shards_in_tenths() {
    let Some(chaos_bolt) = spell(116858) else {
        return;
    };
    assert_eq!(chaos_bolt.power_costs[0].power_type, 7);
    assert_eq!(chaos_bolt.power_costs[0].mana_cost, 20);
}

#[test]
fn crusader_strike_keeps_every_aura_gated_power_cost() {
    let Some(crusader_strike) = spell(35395) else {
        return;
    };
    let costs: Vec<(u8, u32)> = crusader_strike
        .power_costs
        .iter()
        .map(|cost| (cost.order_index, cost.required_aura_spell_id))
        .collect();
    assert_eq!(costs, [(0, 417383), (1, 137029), (2, 137027), (3, 137028)]);
    assert_eq!(crusader_strike.power_costs[1].power_cost_pct, 0.6);
    let categories = crusader_strike.categories.as_ref().unwrap();
    assert_eq!(categories.start_recovery_category, 133);
}

#[test]
fn selfless_healer_stacks_to_four() {
    let Some(selfless_healer) = spell(114250) else {
        return;
    };
    let options = selfless_healer.aura_options.as_ref().unwrap();
    assert_eq!(options.cumulative_aura, 4);
    assert_eq!(options.proc_chance, 101);
}

#[test]
fn recklessness_keeps_all_fifteen_effects() {
    let Some(recklessness) = spell(1719) else {
        return;
    };
    let indexes: Vec<u8> = recklessness.effects.iter().map(|e| e.index).collect();
    assert_eq!(indexes, (0..15).collect::<Vec<u8>>());
}

#[test]
fn crusader_strike_is_a_level_one_paladin_learnable() {
    let Some(paladin) = class(2) else { return };
    assert_eq!(paladin.skill_line, 800);
    let crusader_strike = paladin
        .learnable
        .iter()
        .find(|spell| spell.spell_id == 35395)
        .expect("Crusader Strike not learnable");
    assert_eq!(crusader_strike.required_level, 1);
    assert_eq!(crusader_strike.class_mask, 2);
    assert_eq!(crusader_strike.acquire_method, 2);
    assert_eq!(paladin.power_types, [0, 9, 10, 23, 24, 25]);
}

#[test]
fn retribution_has_highlords_judgment_mastery() {
    let Some(paladin) = class(2) else { return };
    let retribution = paladin
        .specs
        .iter()
        .find(|spec| spec.spec_id == 70)
        .expect("Retribution spec missing");
    assert_eq!(retribution.name, "Retribution");
    assert_eq!(retribution.mastery_spells, [267316]);
    assert_eq!(retribution.role, 2);
    assert_eq!(retribution.spells.len(), 17);
}

#[test]
fn death_knight_lists_runic_power() {
    let Some(death_knight) = class(6) else { return };
    assert!(death_knight.power_types.contains(&6));
}
