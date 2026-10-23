//! Native70448 trait-rank amounts, not spell-name or rank-times-amount rules.
//! TC a352b1fa Player.cpp29422 and SpellInfo.cpp532–553.
use super::test_harness::{Harness, harness_with_plugins, world_db};
use crate::auras::{ActiveAuras, AuraInstance};
use crate::class_progression::{KnownSpells, TraitLoadout, evaluate_player_reference_traits};
use crate::trait_config::Purchase;
use bevy::prelude::DetectChangesMut;
use shared::components::{ActiveSpec, Level, Player};

const SANCTIFIED_PLATES: u32 = 402964;
const OBDURACY: u32 = 385427;

#[derive(serde::Deserialize)]
struct Allocation {
    node_id: u32,
    entry_id: u32,
    active_ranks: u32,
}

fn selected_purchases(
    rank: u32,
    tree: &crate::spell_info::TraitTree,
    omit_late_class: bool,
) -> Vec<Purchase> {
    let allocations: Vec<Allocation> = serde_json::from_str(include_str!(
        "../../tests/fixtures/ret_selected_traits_76f8118.json"
    ))
    .unwrap();
    allocations
        .into_iter()
        .filter(|entry| entry.active_ranks > 0)
        // Native condition20153/group8482 requires23 earlier class points.
        // Lowering two ranks leaves21; the rank1 control omits that late group.
        .filter(|entry| {
            !omit_late_class || !tree.node(entry.node_id).unwrap().groups.contains(&8482)
        })
        .map(|entry| Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: if matches!(entry.node_id, 93009 | 81630) {
                rank
            } else {
                entry.active_ranks
            },
        })
        .collect()
}

fn install_rank(h: &mut Harness, rank: u32, omit_late_class: bool) {
    let identity = h.app.world().get::<Player>(h.caster).unwrap().clone();
    let classes = h.app.world().resource::<crate::spell_info::ClassSpells>();
    let tree = &classes
        .classes
        .iter()
        .find(|class| class.class_id == 2)
        .unwrap()
        .traits;
    let (known, traits) = evaluate_player_reference_traits(
        classes,
        &identity,
        70,
        90,
        &selected_purchases(rank, tree, omit_late_class),
    )
    .expect("exact selected Ret traits, with two valid lower-rank controls");
    for node in [93009, 81630] {
        let actual = traits
            .source_config()
            .0
            .iter()
            .find(|entry| entry.node_id == node)
            .unwrap()
            .total();
        assert_eq!(actual, rank, "native purchased plus granted ranks");
    }
    assert!(known.contains(SANCTIFIED_PLATES));
    assert!(known.contains(OBDURACY));
    h.app
        .world_mut()
        .get_mut::<KnownSpells>(h.caster)
        .unwrap()
        .set_if_neq(known);
    h.app
        .world_mut()
        .get_mut::<TraitLoadout>(h.caster)
        .unwrap()
        .set_if_neq(traits);
    h.advance(3);
}

fn native_rank(rank: u32) -> Harness {
    let mut h = harness_with_plugins(vec![], &[], 2, false, |_| {}).unwrap();
    let native = world_db().unwrap();
    h.app.insert_resource(native.store.clone());
    h.app.insert_resource(native.classes.clone());
    h.app.world_mut().entity_mut(h.caster).insert((
        TraitLoadout::default(),
        ActiveSpec(70),
        Level(90),
    ));
    install_rank(&mut h, rank, rank == 1);
    h
}

fn passive(h: &Harness, spell: u32) -> &AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == h.caster)
        .unwrap_or_else(|| panic!("learned native passive {spell} was not applied"))
}

fn amounts(h: &Harness, spell: u32) -> Vec<f32> {
    passive(h, spell)
        .effects
        .iter()
        .map(|effect| effect.amount)
        .collect()
}

#[test]
fn native_trait_points_rank_two_multiply_reaches_actual_passive() {
    let h = native_rank(2);
    // Definition120041 rows22216–22218, curves64686/64685/64684:
    // rank2 Y=2, raw SpellEffect amounts5/-3/10 =>10/-6/20.
    assert_eq!(amounts(&h, SANCTIFIED_PLATES), [10.0, -6.0, 20.0]);
}

#[test]
fn native_trait_points_rank_two_set_reaches_actual_passive() {
    let h = native_rank(2);
    // Definition107627 rows24203–24205, curves94680/94679/94678:
    // rank2 Y=4/-4/4, independently of raw SpellEffect amounts2/-2/2.
    assert_eq!(amounts(&h, OBDURACY), [4.0, -4.0, 4.0]);
}

#[test]
fn native_trait_points_rank_one_preserves_both_operations() {
    let h = native_rank(1);
    // CurvePoint180714–180719 and284380–284385 contain rank1 X=1.
    assert_eq!(amounts(&h, SANCTIFIED_PLATES), [5.0, -3.0, 10.0]);
    assert_eq!(amounts(&h, OBDURACY), [2.0, -2.0, 2.0]);
}

#[test]
fn native_trait_points_existing_passives_follow_rank_changes() {
    let mut h = native_rank(1);
    // Native spec70 baseline137027 must not be removed/reapplied with changed talents.
    let unchanged_instance = passive(&h, 137027).instance_id;
    assert_eq!(amounts(&h, SANCTIFIED_PLATES), [5.0, -3.0, 10.0]);
    // Keep the same early-class spell book: only the two native ranks change.
    let known_before: Vec<_> = h
        .app
        .world()
        .get::<KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .collect();
    install_rank(&mut h, 2, true);
    let known_after: Vec<_> = h
        .app
        .world()
        .get::<KnownSpells>(h.caster)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(known_before, known_after);
    assert_eq!(passive(&h, 137027).instance_id, unchanged_instance);
    assert_eq!(amounts(&h, SANCTIFIED_PLATES), [10.0, -6.0, 20.0]);
    assert_eq!(amounts(&h, OBDURACY), [4.0, -4.0, 4.0]);
    install_rank(&mut h, 1, true);
    assert_eq!(passive(&h, 137027).instance_id, unchanged_instance);
    assert_eq!(amounts(&h, SANCTIFIED_PLATES), [5.0, -3.0, 10.0]);
    assert_eq!(amounts(&h, OBDURACY), [2.0, -2.0, 2.0]);
}
