//! Real-DB tests on the Paladin trait tree 790 (data/world.db, DB2
//! 12.1.0.69933).

use std::sync::OnceLock;

use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::spell_info::{ClassSpells, load_class_spells};

const PALADIN: u8 = 2;
const SPEC_RETRIBUTION: u32 = 70;
const CLASS_POINTS: u32 = 2801;
const SPEC_POINTS: u32 = 2800;

/// Retribution spec nodes (node, entry): Blade of Justice 184575 is a root;
/// Divine Storm 53385 and Expurgation 383344 hang off it; Avenging Wrath
/// 31884 needs Expurgation; the Swift Justice / Light of Justice choice
/// needs Divine Storm; Final Verdict needs that choice; Jurisdiction sits
/// behind the 8-spec-point gate (group 8478, condition 20152).
const BLADE_OF_JUSTICE: (u32, u32) = (81526, 102498);
const DIVINE_STORM: (u32, u32) = (81527, 102499);
const EXPURGATION: (u32, u32) = (92689, 114830);
const AVENGING_WRATH: (u32, u32) = (81544, 102519);
const SWIFT_JUSTICE: (u32, u32) = (81521, 102491);
const LIGHT_OF_JUSTICE: (u32, u32) = (81521, 114828);
const FINAL_VERDICT: (u32, u32) = (81532, 102504);
const JURISDICTION: (u32, u32) = (81542, 102516);
/// Hero talent choice visible to Retribution: Templar (sub-tree 48) or
/// Herald of the Sun (50).
const HERO_TEMPLAR: (u32, u32) = (99837, 123357);
/// Templar Shake the Heavens 431533, below the granted keystone 95180.
const SHAKE_THE_HEAVENS: (u32, u32) = (95187, 117823);
/// Herald of the Sun Luminosity, below the granted keystone 95099.
const LUMINOSITY: (u32, u32) = (95080, 117677);

fn classes() -> Option<&'static ClassSpells> {
    static CLASSES: OnceLock<Option<ClassSpells>> = OnceLock::new();
    CLASSES
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            let conn =
                Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            Some(load_class_spells(&conn).expect("load class spells"))
        })
        .as_ref()
}

fn paladin_tree() -> Option<&'static TraitTree> {
    let class = classes()?
        .classes
        .iter()
        .find(|class| class.class_id == PALADIN)
        .unwrap();
    Some(&class.traits)
}

#[test]
fn talent_starter_authored_rows_use_cumulative_ranks_and_independent_budgets() {
    let classes = classes().unwrap();
    for (class_id, spec_id) in [(2, 70), (8, 62)] {
        let tree = &classes
            .classes
            .iter()
            .find(|class| class.class_id == class_id)
            .unwrap()
            .traits;
        for (level, expected) in [(10, 1), (11, 2), (20, 11), (70, 61), (80, 61), (90, 68)] {
            let ctx = TraitContext { spec_id, level };
            let purchases = starter_purchases(tree, ctx)
                .unwrap_or_else(|reason| panic!("spec{spec_id} level{level}: {reason}"));
            assert_eq!(
                purchases.iter().map(|purchase| purchase.ranks).sum::<u32>(),
                expected,
                "spec{spec_id} level{level}"
            );
            let entries = build_config(tree, ctx, &purchases).unwrap();
            assert!(
                unspent(tree, ctx, &entries)
                    .iter()
                    .all(|(_, left)| *left >= 0)
            );
        }
    }
    assert!(
        starter_purchases(
            paladin_tree().unwrap(),
            TraitContext {
                spec_id: 62,
                level: 80
            }
        )
        .is_err()
    );
}

fn ret(level: u8) -> TraitContext {
    TraitContext {
        spec_id: SPEC_RETRIBUTION,
        level,
    }
}

fn buy(nodes: &[(u32, u32)]) -> Vec<Purchase> {
    nodes
        .iter()
        .map(|&(node_id, entry_id)| Purchase {
            node_id,
            entry_id,
            ranks: 1,
        })
        .collect()
}

fn entry_ids(entries: &[ConfigEntry]) -> Vec<(u32, u32, u32, u32)> {
    entries
        .iter()
        .map(|e| (e.node_id, e.entry_id, e.ranks, e.granted))
        .collect()
}

#[test]
fn paladin_tree_790_has_five_currencies_and_three_hero_trees() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    assert_eq!(tree.tree_id, 790);
    let currencies: Vec<u32> = tree.currencies.iter().map(|c| c.id).collect();
    assert_eq!(currencies, vec![2801, 2800, 2986, 2987, 2988]);
    let sub_trees: Vec<(u32, &str)> = tree
        .sub_trees
        .iter()
        .map(|sub| (sub.id, sub.name.as_str()))
        .collect();
    assert_eq!(
        sub_trees,
        vec![
            (48, "Templar"),
            (49, "Lightsmith"),
            (50, "Herald of the Sun")
        ]
    );
    let hero = tree.node(HERO_TEMPLAR.0).unwrap();
    assert_eq!(
        hero.kind,
        crate::spell_info::TraitNodeType::SubTreeSelection
    );
    assert_eq!(hero.entry(HERO_TEMPLAR.1).unwrap().sub_tree_id, 48);
    let storm = tree.node(DIVINE_STORM.0).unwrap();
    assert_eq!((storm.pos_x, storm.pos_y), (12600, 2100));
    assert_eq!(storm.entry(DIVINE_STORM.1).unwrap().spell_id, 53385);
}

#[test]
fn points_per_level_follow_currency_sources() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let budget = |level| unspent(tree, ret(level), &[]);
    // Class points at even levels 10-70 and 82/85/88; spec points at odd
    // levels 11-69 and 81/84/87/90; each hero currency at 71-80 and 83/86/89.
    assert_eq!(
        budget(9),
        vec![(2801, 0), (2800, 0), (2986, 0), (2987, 0), (2988, 0)]
    );
    assert_eq!(
        budget(11),
        vec![(2801, 1), (2800, 1), (2986, 0), (2987, 0), (2988, 0)]
    );
    assert_eq!(
        budget(80),
        vec![(2801, 31), (2800, 30), (2986, 10), (2987, 10), (2988, 10)]
    );
    assert_eq!(
        budget(90),
        vec![(2801, 34), (2800, 34), (2986, 13), (2987, 13), (2988, 13)]
    );
}

#[test]
fn ret_level_80_empty_config_is_valid_with_granted_nodes_only() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let entries = build_config(tree, ret(80), &[]).expect("empty config");
    // Retribution group 8486 grants Hammer of Wrath 81510, Auras of the
    // Resolute 81600 and Greater Judgment 81603; group 10536 grants the hero
    // keystones Light's Guidance 95180 (Templar) and Dawnlight 95099 (Herald)
    // from level 71.
    assert_eq!(
        entry_ids(&entries),
        vec![
            (81510, 133481, 0, 1),
            (81600, 102587, 0, 1),
            (81603, 102590, 0, 1),
            (95099, 117696, 0, 1),
            (95180, 117813, 0, 1),
        ]
    );
    let spells: Vec<u32> = talent_spell_entries(tree, &entries)
        .map(|entry| entry.spell_id)
        .collect();
    // No hero tree selected: keystone spells do not apply.
    assert_eq!(spells, vec![1241288, 385633, 231663]);
}

#[test]
fn node_without_its_prerequisite_is_rejected() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let error = build_config(tree, ret(80), &buy(&[BLADE_OF_JUSTICE, AVENGING_WRATH])).unwrap_err();
    assert_eq!(
        error,
        "node 81544 requires one of nodes [92689] fully ranked"
    );
}

#[test]
fn spending_more_spec_points_than_the_level_grants_is_rejected() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let error = build_config(tree, ret(11), &buy(&[BLADE_OF_JUSTICE, DIVINE_STORM])).unwrap_err();
    assert_eq!(
        error,
        format!("config spends 2 points of currency {SPEC_POINTS}, level 11 has 1")
    );
    assert!(build_config(tree, ret(13), &buy(&[BLADE_OF_JUSTICE, DIVINE_STORM])).is_ok());
}

#[test]
fn choice_node_with_two_entries_is_rejected() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let purchases = buy(&[
        BLADE_OF_JUSTICE,
        DIVINE_STORM,
        SWIFT_JUSTICE,
        LIGHT_OF_JUSTICE,
    ]);
    let error = build_config(tree, ret(80), &purchases).unwrap_err();
    assert_eq!(
        error,
        "choice node 81521 must have exactly one entry, got 2"
    );
}

#[test]
fn gated_node_needs_eight_spec_points_spent_above_the_gate() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let chain = [
        BLADE_OF_JUSTICE,
        DIVINE_STORM,
        SWIFT_JUSTICE,
        FINAL_VERDICT,
        JURISDICTION,
    ];
    let error = build_config(tree, ret(80), &buy(&chain)).unwrap_err();
    assert_eq!(
        error,
        "node 81542 is not available: condition 20152 requires 8 points of currency 2800 spent in earlier rows, 4 spent"
    );
    // Four more rows-above-the-gate nodes: Expurgation, Avenging Wrath,
    // Judgment of Justice 93161 and Righteous Cause 81523 (entry 102494).
    let mut unlocked = chain.to_vec();
    unlocked.extend([
        EXPURGATION,
        AVENGING_WRATH,
        (93161, 115440),
        (81523, 102494),
    ]);
    assert!(build_config(tree, ret(80), &buy(&unlocked)).is_ok());
}

#[test]
fn hero_tree_before_level_71_is_rejected() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let error = build_config(tree, ret(70), &buy(&[HERO_TEMPLAR])).unwrap_err();
    assert_eq!(
        error,
        "node 99837 is not available: condition 27046 requires level 71"
    );
    assert!(build_config(tree, ret(71), &buy(&[HERO_TEMPLAR])).is_ok());
}

#[test]
fn hero_nodes_need_their_tree_selected() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let error = build_config(tree, ret(80), &buy(&[HERO_TEMPLAR, LUMINOSITY])).unwrap_err();
    assert_eq!(
        error,
        "node 95080 belongs to hero talent tree 50, which is not selected"
    );
    let entries =
        build_config(tree, ret(80), &buy(&[HERO_TEMPLAR, SHAKE_THE_HEAVENS])).expect("templar");
    assert_eq!(selected_sub_tree(tree, &entries), Some(48));
    let spells: Vec<u32> = talent_spell_entries(tree, &entries)
        .map(|entry| entry.spell_id)
        .collect();
    // Templar keystone Light's Guidance 427445 and Shake the Heavens apply;
    // the Herald keystone Dawnlight 431377 does not.
    assert!(spells.contains(&427445));
    assert!(spells.contains(&431533));
    assert!(!spells.contains(&431377));
    let hero_left = unspent(tree, ret(80), &entries);
    assert_eq!(hero_left[..2], [(CLASS_POINTS, 31), (SPEC_POINTS, 30)]);
}

#[test]
fn valid_ret_build_applies_its_talent_spells() {
    let Some(tree) = paladin_tree() else {
        return;
    };
    let build = [BLADE_OF_JUSTICE, DIVINE_STORM, EXPURGATION, AVENGING_WRATH];
    let entries = build_config(tree, ret(80), &buy(&build)).expect("ret build");
    let spells: Vec<u32> = talent_spell_entries(tree, &entries)
        .map(|entry| entry.spell_id)
        .collect();
    for spell_id in [184575, 53385, 383344, 31884] {
        assert!(spells.contains(&spell_id), "missing {spell_id}");
    }
    assert_eq!(unspent(tree, ret(80), &entries)[1], (SPEC_POINTS, 26));
}
