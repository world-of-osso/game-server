//! Real-DB tests against data/world.db (content_item + DB2 scaling tables);
//! skip when the DB is absent. Retail tooltips: Wowhead tooltip API
//! (`nether.wowhead.com/tooltip/item/<id>?dataEnv=1`), fetched 2026-09-24.

use std::path::PathBuf;

use super::*;
use crate::item_templates::{ItemTemplates, load_item_templates};

const NOTCHED_SHORTSWORD: u32 = 727;
const VENDETTA: u32 = 776;
const BLACK_DUSKWOOD_STAFF: u32 = 937;
const FORLORN_BREASTPLATE_OF_WAR: u32 = 35588;

fn world_data() -> Option<(ItemTemplates, ItemScalingTables)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
    Some((
        load_item_templates(&path).unwrap(),
        load_item_scaling_tables(&path).unwrap(),
    ))
}

/// `item_id` fixed at `item_level` with no squish (Wowhead shows some items
/// at a scaled level; the formula is checked at that level).
fn at_item_level(templates: &ItemTemplates, item_id: u32, item_level: u16) -> ItemTemplate {
    let mut template = *templates.get(item_id).unwrap();
    template.scaling.item_level = item_level;
    template.scaling.squish_era = u8::MAX;
    template
}

fn primary(strength: f32, agility: f32, stamina: f32, intellect: f32) -> UnitStats {
    UnitStats {
        strength,
        agility,
        stamina,
        intellect,
        spirit: 0.0,
    }
}

#[test]
fn black_duskwood_staff_squishes_to_12_with_tooltip_damage() {
    let Some((templates, tables)) = world_data() else {
        return;
    };
    // ItemSparse ItemLevel 21 → Midnight curve 92181 (10→10, 342→70) → 12.
    // Tooltip: Item Level 12, "6 - 9 Damage", Speed 3.60.
    let stats = item_stats(templates.get(BLACK_DUSKWOOD_STAFF).unwrap(), &tables);
    assert_eq!(stats.item_level, 12);
    assert_eq!(
        stats.weapon,
        Some(WeaponDamage {
            min: 6.0,
            max: 9.0,
            speed: 3.6,
            kind: WeaponType::TwoHand,
        })
    );
    assert_eq!(stats.block, ItemStatBlock::default());
}

#[test]
fn notched_shortsword_matches_tooltip() {
    let Some((templates, tables)) = world_data() else {
        return;
    };
    // Tooltip: Item Level 5, "1 - 2 Damage", Speed 2.60, +1 Agility, +1 Stamina.
    let stats = item_stats(templates.get(NOTCHED_SHORTSWORD).unwrap(), &tables);
    assert_eq!(stats.item_level, 5);
    assert_eq!(stats.block.primary, primary(0.0, 1.0, 1.0, 0.0));
    assert_eq!(
        stats.weapon,
        Some(WeaponDamage {
            min: 1.0,
            max: 2.0,
            speed: 2.6,
            kind: WeaponType::OneHand,
        })
    );
}

#[test]
fn vendetta_at_tooltip_level_83_has_rating_and_damage() {
    let Some((templates, tables)) = world_data() else {
        return;
    };
    // Tooltip: Item Level 83, "13 - 23 Damage", Speed 1.80, +13 Agility, +13 Critical Strike.
    // Crit: 7889 × RandPropPoints(83).SuperiorF_3 × 0.0001 × gtCombatRatingsMultByILvl weapon.
    let stats = item_stats(&at_item_level(&templates, VENDETTA, 83), &tables);
    assert_eq!(stats.block.primary, primary(0.0, 13.0, 0.0, 0.0));
    assert_eq!(stats.block.secondary.crit, 13.0);
    assert_eq!(
        stats.weapon,
        Some(WeaponDamage {
            min: 13.0,
            max: 23.0,
            speed: 1.8,
            kind: WeaponType::Dagger,
        })
    );
}

/// No content item carries a tertiary stat, so Vendetta's crit allocation stands in for
/// each: ITEM_MOD_CR_SPEED 61, CR_LIFESTEAL 62 and CR_AVOIDANCE 63 are combat ratings
/// (SimC `util::is_combat_rating`) and scale like its 13 Critical Strike.
#[test]
fn tertiary_allocations_scale_as_combat_ratings() {
    let Some((templates, tables)) = world_data() else {
        return;
    };
    let vendetta = at_item_level(&templates, VENDETTA, 83);
    let crit = vendetta
        .scaling
        .stats
        .iter()
        .position(|alloc| alloc.stat == 32)
        .unwrap();
    let with_stat = |stat: i8| {
        let mut template = vendetta;
        template.scaling.stats[crit].stat = stat;
        item_stats(&template, &tables).block.secondary
    };
    assert_eq!(with_stat(61).speed, 13.0);
    assert_eq!(with_stat(62).leech, 13.0);
    assert_eq!(with_stat(63).avoidance, 13.0);
    assert_eq!(with_stat(62).crit, 0.0);
}

#[test]
fn forlorn_breastplate_of_war_stats_and_armor() {
    let Some((templates, tables)) = world_data() else {
        return;
    };
    // Tooltip (Item Level 34): 26 Armor, +8 [Strength or Intellect], +12 Stamina, +15 Critical Strike.
    let tooltip = item_stats(
        &at_item_level(&templates, FORLORN_BREASTPLATE_OF_WAR, 34),
        &tables,
    );
    assert_eq!(tooltip.block.primary, primary(8.0, 0.0, 12.0, 8.0));
    assert_eq!(
        (tooltip.block.secondary.crit, tooltip.block.secondary.armor),
        (15.0, 26.0)
    );
    // Unscaled: ItemLevel 35 squishes to 15 (budget SuperiorF_0 8.045):
    // Str/Int 5259 → 4, Sta 7889 → 6, crit 6777 × 1.5 → 8, plate armor 18.
    let base = item_stats(templates.get(FORLORN_BREASTPLATE_OF_WAR).unwrap(), &tables);
    assert_eq!(base.item_level, 15);
    assert_eq!(base.block.primary, primary(4.0, 0.0, 6.0, 4.0));
    assert_eq!(
        (base.block.secondary.crit, base.block.secondary.armor),
        (8.0, 18.0)
    );
    assert_eq!(base.weapon, None);
}
