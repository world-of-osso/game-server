//! Native target selection and aggro, independent of follower kit admission.
use super::test_harness::{Harness, harness, spawn_npc};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::CombatStatus;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[
            375576,
            50286,
            61882,
            53385,
            188443,
            DIVINE_RESONANCE,
            POLYMORPH,
        ],
        2,
    )
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

/// Divine Resonance 386731: Dummy effects on TARGET_UNIT_DEST_AREA_ENEMY (31) with
/// SPELL_ATTR1_NO_THREAT (Attributes_1 0x400), so `HasInitialAggro()` is false.
const DIVINE_RESONANCE: u32 = 386731;
const POLYMORPH: u32 = 118;

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

/// TC a352b1fa Spell.cpp:2992-2996 calls `AtTargetAttacked(unit, HasInitialAggro())`
/// for a landed harmful hit; Unit.cpp:9281-9287 engages an unengaged target only with
/// initial aggro. Divine Resonance's Dummy effects are positive (SpellInfo.cpp:4612-5067
/// has no enemy-target rule for Dummy), so its area hit engages nobody.
#[test]
fn hits_without_initial_aggro_engage_only_an_already_engaged_enemy() {
    let (mut h, untouched) = fixture();
    let caster = h.caster;
    h.app
        .init_resource::<super::test_harness::ObservedSelectedTargets>();
    let threatens = |h: &Harness, unit: Entity| {
        h.app
            .world()
            .get::<shared::components::ThreatTable>(unit)
            .is_some_and(|table| {
                table
                    .entries()
                    .iter()
                    .any(|entry| entry.entity == caster.to_bits())
            })
    };
    super::creature_cast(
        h.app.world_mut(),
        caster,
        DIVINE_RESONANCE,
        Some(caster),
        true,
    )
    .unwrap();
    let hit_untouched = h
        .app
        .world()
        .resource::<super::test_harness::ObservedSelectedTargets>()
        .0
        .iter()
        .any(|(source, spell, units)| {
            *source == caster && *spell == DIVINE_RESONANCE && units.contains(&untouched)
        });
    assert!(
        hit_untouched,
        "the native enemy area reaches the untouched unit"
    );
    assert!(
        !h.app
            .world()
            .get::<CombatStatus>(untouched)
            .is_some_and(|status| status.0),
        "a hit without initial aggro must not engage an unengaged unit"
    );
    assert!(!threatens(&h, untouched));
    // Polymorph 118 is harmful with SPELL_ATTR1_NO_THREAT: it engages only an engaged target.
    assert!(!threatens(&h, h.npc));
    let npc = h.npc;
    super::creature_cast(h.app.world_mut(), caster, POLYMORPH, Some(npc), true).unwrap();
    assert!(
        threatens(&h, npc),
        "a harmful hit without initial aggro still engages an already engaged unit"
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
