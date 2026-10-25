//! Real-DB tests against data/world.db (content_item + DB2 scaling tables).
//! Retail tooltips: Wowhead tooltip API
//! (`nether.wowhead.com/tooltip/item/<id>?dataEnv=1`), fetched 2026-09-24.

use super::*;
use crate::item_templates::{ItemTemplates, load_item_templates};

// Exact headers and level-1 rows from scripts/tests/fixtures/forever-scaling-70205;
// ArmorLocation uses chest row 5. Embedded because desktop snapshots omit CSVs.
const FOREVER_SCALING_CSVS: [(&str, &str); 9] = [
    (
        "rand_prop_points",
        "ID,GoodF_0,GoodF_1,GoodF_2,GoodF_3,GoodF_4,SuperiorF_0,SuperiorF_1,SuperiorF_2,SuperiorF_3,SuperiorF_4,EpicF_0,EpicF_1,EpicF_2,EpicF_3,EpicF_4\n1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1",
    ),
    (
        "armor_location",
        "ID,Clothmodifier,Leathermodifier,Chainmodifier,Platemodifier\n5,0.159999996,0.159999996,0.159999996,0.159999996",
    ),
    (
        "item_armor_quality",
        "ID,Qualitymod_0,Qualitymod_1,Qualitymod_2,Qualitymod_3,Qualitymod_4,Qualitymod_5,Qualitymod_6\n1,0.899999976,0.949999988,1,1.10000002,1.20000005,1.20000005,1.20000005",
    ),
    (
        "item_armor_shield",
        "ItemLevel,Quality_0,Quality_1,Quality_2,Quality_3,Quality_4,Quality_5,Quality_6\n1,5,10,16,27,38,38,38",
    ),
    (
        "item_armor_total",
        "ItemLevel,Cloth,Leather,Mail,Plate\n1,18.5200005,108.660004,241.630005,271.579987",
    ),
    (
        "item_damage_one_hand",
        "ItemLevel,Quality_0,Quality_1,Quality_2,Quality_3,Quality_4,Quality_5,Quality_6\n1,0.761455715,0.780158043,1.2690928,3.89907432,6.95736933,6.95736933,6.95736933",
    ),
    (
        "item_damage_one_hand_caster",
        "ItemLevel,Quality_0,Quality_1,Quality_2,Quality_3,Quality_4,Quality_5,Quality_6\n1,0.761455715,0.780158043,1.2690928,3.89907432,6.95736933,6.95736933,6.95736933",
    ),
    (
        "item_damage_two_hand",
        "ItemLevel,Quality_0,Quality_1,Quality_2,Quality_3,Quality_4,Quality_5,Quality_6\n1,0.989892423,1.01420546,1.64982069,5.06879663,9.04458046,9.04458046,9.04458046",
    ),
    (
        "item_damage_two_hand_caster",
        "ItemLevel,Quality_0,Quality_1,Quality_2,Quality_3,Quality_4,Quality_5,Quality_6\n1,0.989892423,1.01420546,1.64982069,5.06879663,9.04458046,9.04458046,9.04458046",
    ),
];

fn pinned_scaling_database() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    let conn = Connection::open(&path).unwrap();
    for (table, csv) in FOREVER_SCALING_CSVS {
        let mut lines = csv.lines();
        let columns = lines.next().unwrap();
        let definitions = columns
            .split(',')
            .enumerate()
            .map(|(index, column)| {
                let kind = if index == 0 { "INTEGER" } else { "REAL" };
                format!("{column} {kind}")
            })
            .collect::<Vec<_>>()
            .join(",");
        conn.execute_batch(&format!("CREATE TABLE skyborne_{table} ({definitions});"))
            .unwrap();
        for line in lines {
            let values: Vec<f64> = line
                .split(',')
                .map(|value| value.parse().unwrap())
                .collect();
            let parameters = vec!["?"; values.len()].join(",");
            conn.execute(
                &format!("INSERT INTO skyborne_{table} VALUES ({parameters})"),
                rusqlite::params_from_iter(values),
            )
            .unwrap();
        }
        conn.execute_batch(&format!(
            "CREATE TABLE {table} AS SELECT * FROM skyborne_{table};"
        ))
        .unwrap();
    }
    // Retail-only capability with deliberately different values at the same level.
    conn.execute_batch(
        "ALTER TABLE rand_prop_points ADD COLUMN DamageReplaceStatF REAL;
         ALTER TABLE rand_prop_points ADD COLUMN DamageSecondaryF REAL;
         UPDATE rand_prop_points SET DamageReplaceStatF = 41.25, DamageSecondaryF = 73.5,
             SuperiorF_0 = 19, GoodF_0 = 29;
         UPDATE item_armor_total SET Cloth = 1000;
         UPDATE item_damage_one_hand SET Quality_2 = 50;
         CREATE TABLE item_squish_era (ID INTEGER, CurveID INTEGER);",
    )
    .unwrap();
    (directory, path)
}

#[test]
fn pinned_70205_scaling_loads_normal_stats_without_retail_spell_budgets() {
    let (_directory, path) = pinned_scaling_database();
    let retail = load_item_scaling_tables(&path).unwrap();
    assert_eq!(retail.spell_budgets[&1], [41.25, 73.5, 19.0]);
    let forever = load_forever_item_scaling_tables(&path).unwrap();
    assert!(forever.spell_budgets.is_empty());
    assert_eq!(forever.squish, None);
    let mut armor = ItemTemplate {
        inventory_type: 5,
        scaling: ItemScaling {
            class_id: ITEM_CLASS_ARMOR,
            subclass_id: 1,
            quality: 2,
            item_level: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    armor.scaling.stats[0] = ItemStatAllocation {
        stat: 4,
        allocation: 10000,
        ..Default::default()
    };
    let stats = item_stats(&armor, &forever);
    assert_eq!(stats.block.primary.strength, 1.0);
    assert_eq!(stats.block.secondary.armor, 3.0);
    let retail_stats = item_stats(&armor, &retail);
    assert_eq!(retail_stats.block.primary.strength, 29.0);
    assert_eq!(retail_stats.block.secondary.armor, 160.0);
    let weapon = ItemTemplate {
        inventory_type: 13,
        scaling: ItemScaling {
            class_id: ITEM_CLASS_WEAPON,
            quality: 2,
            item_level: 1,
            delay_ms: 2000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        item_stats(&weapon, &forever).weapon,
        Some(WeaponDamage {
            min: 2.0,
            max: 3.0,
            speed: 2.0,
            kind: WeaponType::OneHand,
        })
    );
    assert_eq!(item_stats(&weapon, &retail).weapon.unwrap().min, 100.0);
}

#[test]
fn pinned_70205_scaling_rejects_missing_source_owned_tables() {
    let (_directory, path) = pinned_scaling_database();
    Connection::open(&path)
        .unwrap()
        .execute_batch("DROP TABLE skyborne_item_damage_one_hand;")
        .unwrap();
    let error = load_forever_item_scaling_tables(&path).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("no such table: skyborne_item_damage_one_hand"),
        "{error}"
    );
}

#[test]
fn pinned_70205_scaling_does_not_make_retail_budget_fields_optional() {
    let (_directory, path) = pinned_scaling_database();
    Connection::open(&path)
        .unwrap()
        .execute_batch("ALTER TABLE rand_prop_points DROP COLUMN DamageSecondaryF;")
        .unwrap();
    let error = load_item_scaling_tables(&path).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("no such column: DamageSecondaryF"),
        "{error}"
    );
}

const NOTCHED_SHORTSWORD: u32 = 727;
const VENDETTA: u32 = 776;
const BLACK_DUSKWOOD_STAFF: u32 = 937;
const FORLORN_BREASTPLATE_OF_WAR: u32 = 35588;

fn world_data() -> Option<(ItemTemplates, ItemScalingTables)> {
    let path = crate::test_data::world_db_path();
    Some((
        load_item_templates(&path).unwrap(),
        load_item_scaling_tables(&path).unwrap(),
    ))
}

/// `item_id` fixed at `item_level` with no squish (Wowhead shows some items
/// at a scaled level; the formula is checked at that level).
fn at_item_level(templates: &ItemTemplates, item_id: u32, item_level: u16) -> ItemTemplate {
    let mut template = *templates
        .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
        .unwrap();
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
    let stats = item_stats(
        templates
            .get(
                shared::item_data::ItemDefinitionSource::Retail,
                BLACK_DUSKWOOD_STAFF,
            )
            .unwrap(),
        &tables,
    );
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
    let stats = item_stats(
        templates
            .get(
                shared::item_data::ItemDefinitionSource::Retail,
                NOTCHED_SHORTSWORD,
            )
            .unwrap(),
        &tables,
    );
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
    let base = item_stats(
        templates
            .get(
                shared::item_data::ItemDefinitionSource::Retail,
                FORLORN_BREASTPLATE_OF_WAR,
            )
            .unwrap(),
        &tables,
    );
    assert_eq!(base.item_level, 15);
    assert_eq!(base.block.primary, primary(4.0, 0.0, 6.0, 4.0));
    assert_eq!(
        (base.block.secondary.crit, base.block.secondary.armor),
        (8.0, 18.0)
    );
    assert_eq!(base.weapon, None);
}
