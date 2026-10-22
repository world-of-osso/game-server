//! TRAIT_VARIANT_FAMILY_FROM_SIMC: actual selected native loadouts and kit casts.
//! SimC76f8118 mage3891 and druid4457–4460; DB2 OverridesSpellID is zero.
use super::kit_tests::at_level;
use super::{Follower, FollowerCatalog};
use crate::class_progression::{TraitLoadout, evaluate_reference_traits};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use crate::spell_info::ClassSpells;
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const FIRE: &str = include_str!("../../tests/fixtures/trait_variant_fire.json");
const FERAL: &str = include_str!("../../tests/fixtures/trait_variant_feral.json");

#[derive(serde::Deserialize)]
struct SelectedPurchase {
    node_id: u32,
    entry_id: u32,
    ranks: u32,
}

fn read_purchases(input: &str) -> Vec<crate::trait_config::Purchase> {
    let entries: Vec<SelectedPurchase> = serde_json::from_str(input).unwrap();
    entries
        .into_iter()
        .map(|entry| crate::trait_config::Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.ranks,
        })
        .collect()
}

fn selected_actor(entry: u32, purchases: &str, action: u32) -> (Harness, Entity) {
    selected_actor_with_purchases(entry, read_purchases(purchases), action)
}

fn selected_actor_with_purchases(
    entry: u32,
    purchases: Vec<crate::trait_config::Purchase>,
    action: u32,
) -> (Harness, Entity) {
    let (mut h, actor) = at_level(entry, 90, |_| {});
    let world = h.app.world_mut();
    let profile = world.get::<Follower>(actor).unwrap().profile.clone();
    let (known, traits) =
        evaluate_reference_traits(world.resource::<ClassSpells>(), &profile, 90, &purchases)
            .expect("full allocated fixture must pass native grants, edges and budgets");
    world.entity_mut(actor).insert((known, traits));
    world
        .resource_mut::<FollowerCatalog>()
        .kits
        .retain(|row| row.spec_id != profile.spec_id || row.spell_id == action);
    (h, actor)
}

fn selected_spell(h: &Harness, actor: Entity, spell: u32) -> bool {
    h.app
        .world()
        .get::<TraitLoadout>(actor)
        .unwrap()
        .active_talent_spells()
        .any(|entry| entry.spell_id == spell)
}

fn drive(h: &mut Harness, ticks: usize) {
    for _ in 0..ticks {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
}

fn casts(h: &Harness, actor: Entity) -> Vec<u32> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::CastSuccess && event.source == Some(actor.to_bits())
        })
        .filter_map(|event| event.spell_id)
        .collect()
}

#[test]
#[ignore = "native selected-reference loadouts; run with frozen/current native DB view"]
fn trait_variant_selected_lunar_inspiration_casts_and_maintains_feral_moonfire() {
    let (mut h, actor) = selected_actor(232800, FERAL, 8921);
    assert!(selected_spell(&h, actor, 155580));
    let first = h.inbox.log.len();
    drive(&mut h, 120);
    let actions = casts(&h, actor);
    assert!(
        actions.contains(&155625),
        "selected Lunar Inspiration must cast155625 in Cat, not base8921: {actions:?}"
    );
    assert!(!actions.contains(&8921));
    assert_eq!(actions.iter().filter(|&&spell| spell == 155625).count(), 1);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 155625));
    assert!(h.inbox.log[first..].iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.source == Some(actor.to_bits())
            && event.target == Some(h.npc.to_bits())
            && event.spell_id == Some(155625)
            && event.amount > 0
    }));
}

fn assert_flamestrike_choice(selected_entry: u32, selected: u32, unselected: u32) {
    let mut purchases = read_purchases(FIRE);
    purchases
        .iter_mut()
        .find(|purchase| purchase.node_id == 109409)
        .unwrap()
        .entry_id = selected_entry;
    let (mut h, actor) = selected_actor_with_purchases(209059, purchases, 1254851);
    assert!(selected_spell(&h, actor, selected));
    assert!(!selected_spell(&h, actor, unselected));
    let second = spawn_npc(&mut h.app, 1.0);
    let third = spawn_npc(&mut h.app, 2.0);
    for target in [h.npc, second, third] {
        crate::combat_refs::set_in_combat_with(h.app.world_mut(), h.caster, target);
    }
    crate::spell_cast::cast_triggered(h.app.world_mut(), actor, 48108, Some(actor), 0);
    drive(&mut h, 100);
    let actions = casts(&h, actor);
    assert!(
        actions.contains(&selected),
        "native selected{selected} must replace the kit's unselected{unselected}: {actions:?}"
    );
    assert!(!actions.contains(&unselected));
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::Damage
            && event.source == Some(actor.to_bits())
            && event.spell_id == Some(selected)
            && event.amount > 0
    }));
}

#[test]
#[ignore = "native selected-reference loadouts; run with frozen/current native DB view"]
fn trait_variant_selected_flamestrike_casts_selected2120_not_unselected1254851() {
    assert_flamestrike_choice(135619, 2120, 1254851);
}

#[test]
#[ignore = "native selected-reference loadouts; run with frozen/current native DB view"]
fn trait_variant_selected_flamestrike_retains_selected1254851_not_sibling2120() {
    assert_flamestrike_choice(135602, 1254851, 2120);
}
