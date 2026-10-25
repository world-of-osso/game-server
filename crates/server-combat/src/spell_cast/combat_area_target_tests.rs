//! Native target selection and aggro, independent of follower kit admission.
use super::test_harness::{Harness, harness, spawn_npc};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::CombatStatus;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[375576, 50286, 61882, 53385, 188443], 2)
        .expect("native combat-area spell data");
    // Put the untouched unit nearer than the engaged one: filter before target caps.
    let untouched = spawn_npc(&mut h.app, 1.0);
    let mode = crate::test_data::world_data().templates[&6].aggro_mode;
    {
        let mut commands = h.app.world_mut().commands();
        for unit in [h.npc, untouched] {
            crate::creature_aggro::insert_aggro_components(&mut commands.entity(unit), mode);
        }
    }
    h.app.world_mut().flush();
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), h.caster, h.npc);
    (h, untouched)
}

fn selected(h: &mut Harness, id: u32, index: u8, explicit: Option<Entity>) -> Vec<Entity> {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap();
    super::targets::select_targets(
        h.app.world_mut(),
        h.caster,
        explicit,
        &spell,
        effect,
        super::max_targets(&spell),
        false,
    )
    .expect("native implicit selection")
}

#[test]
fn divine_toll_area_selects_engaged_enemy_not_the_untouched_neighbour() {
    let (mut h, untouched) = fixture();
    let caster = h.caster;
    let units = selected(&mut h, 375576, 1, Some(caster));
    assert!(units.contains(&h.npc), "keep the real engaged enemy");
    assert!(
        !units.contains(&untouched),
        "Divine Toll implicit area must not acquire an untouched pack"
    );
}

#[test]
fn starfall_periodic_area_selects_engaged_enemy_not_the_untouched_neighbour() {
    let (mut h, untouched) = fixture();
    let caster = h.caster;
    let units = selected(&mut h, 50286, 0, Some(caster));
    assert!(units.contains(&h.npc));
    assert!(
        !units.contains(&untouched),
        "191034's native50286 selection must leave untouched enemies alone"
    );
}

#[test]
fn divine_toll_native_cast_does_not_start_a_fresh_pack_without_a_kit_guard() {
    let _rng = crate::random::seed_scope(7);
    let (mut h, untouched) = fixture();
    let caster = h.caster;
    super::creature_cast(h.app.world_mut(), caster, 375576, Some(caster), true).unwrap();
    assert!(
        !h.app
            .world()
            .get::<CombatStatus>(untouched)
            .is_some_and(|status| status.0),
        "ordinary native hit processing must not add zero threat to the untouched neighbour"
    );
    assert!(
        h.app
            .world()
            .get::<CombatStatus>(h.npc)
            .is_some_and(|status| status.0)
    );
}

#[test]
fn earthquake_and_divine_storm_still_select_out_of_combat_enemies() {
    let (mut h, untouched) = fixture();
    let caster = h.caster;
    for (id, index) in [(61882, 3), (53385, 0)] {
        assert!(
            selected(&mut h, id, index, Some(caster)).contains(&untouched),
            "unmarked source spell{id} remains able to pull"
        );
    }
}

#[test]
fn chain_lightning_primary_target_can_be_out_of_combat() {
    let (mut h, untouched) = fixture();
    assert!(selected(&mut h, 188443, 0, Some(untouched)).contains(&untouched));
}
