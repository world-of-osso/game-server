//! Production follower kits and the native spell/script registries.
use super::*;
use combat::followers::{native_fixture, party_unintended_reach};
use shared::components::Level;

fn follower(entry: u32, class: u8) -> (crate::test_support::Harness, Entity) {
    let mut harness = crate::test_support::harness(Vec::new(), &[], class).unwrap();
    let full_store = crate::test_support::world_db()
        .unwrap()
        .spell_store()
        .clone();
    harness.app.insert_resource(full_store);
    harness
        .app
        .world_mut()
        .get_mut::<Level>(harness.caster)
        .unwrap()
        .0 = 32;
    let unit = native_fixture::spawn(harness.app.world_mut(), harness.caster, entry).unwrap();
    (harness, unit)
}

#[test]
fn fire_follower_includes_fire_blast_ignite_spread() {
    let (mut harness, unit) = follower(209059, 8);
    let reaches = party_unintended_reach(harness.app.world_mut(), &[unit]);
    assert_eq!(reaches[0].0, unit);
    assert!(reaches[0].1 >= 40.0, "{reaches:?}");
}

#[test]
fn protection_warrior_follower_includes_melee_cleave() {
    let (mut harness, unit) = follower(215517, 1);
    let reaches = party_unintended_reach(harness.app.world_mut(), &[unit]);
    assert!(reaches[0].1 > 0.0, "{reaches:?}");
}

#[test]
fn elemental_follower_includes_accumulated_chain_lightning_jumps() {
    let (mut harness, unit) = follower(209053, 7);
    let reaches = party_unintended_reach(harness.app.world_mut(), &[unit]);
    assert!(reaches[0].2 >= 20.0, "{reaches:?}");
}

#[test]
fn pet_reach_is_reported_at_the_pet_not_its_owner() {
    use shared::protocol::{ACT_ENABLED, PET_ACTION_BAR_SLOTS, REACT_DEFENSIVE, pet_action_button};
    let (mut harness, owner) = follower(209065, 3);
    let mut bar = [0; PET_ACTION_BAR_SLOTS];
    // Native guardian spell7342 is a 10-yard hostile cone (SpellRadius13).
    bar[3] = pet_action_button(7342, ACT_ENABLED);
    let pet = harness
        .app
        .world_mut()
        .spawn((
            combat::hunter_pet::HunterPet { owner, slot: 0 },
            combat::pet_control::PetCharm::new(bar, REACT_DEFENSIVE),
            combat::maps::InMap::continent(0),
            shared::components::Position {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let off_map = harness
        .app
        .world_mut()
        .spawn((
            combat::hunter_pet::HunterPet { owner, slot: 1 },
            combat::maps::InMap::continent(1),
            shared::components::Position {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[owner]);
    assert!(!reaches.iter().any(|(unit, _, _)| *unit == off_map));
    let pet_reach = reaches.iter().find(|(unit, _, _)| *unit == pet).unwrap();
    assert_eq!(pet_reach.1, 10.0);
    assert_eq!(pet_reach.2, 0.0);
}

#[test]
fn demonology_felguard_autonomous_felstorm_contributes_pet_reach() {
    let (mut harness, owner) = follower(230861, 9);
    let pet = harness
        .app
        .world_mut()
        .spawn((
            combat::hunter_pet::HunterPet { owner, slot: 0 },
            shared::components::Npc {
                template_id: 17252,
                name: "Felguard".into(),
            },
            combat::maps::InMap::continent(0),
            shared::components::Position {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[owner]);
    let pet_reach = reaches.iter().find(|(unit, _, _)| *unit == pet).unwrap();
    assert_eq!(pet_reach.1, 8.0);
}

#[test]
fn transformed_unholy_ghoul_reports_its_autonomous_cleave() {
    let mut harness = crate::test_support::harness(Vec::new(), &[], 6).unwrap();
    harness.app.insert_resource(
        crate::test_support::world_db()
            .unwrap()
            .spell_store()
            .clone(),
    );
    let owner = harness.caster;
    harness.app.world_mut().entity_mut(owner).insert((
        shared::components::ActiveSpec(252),
        combat::class_progression::KnownSpells::from_baseline(vec![1233448]),
    ));
    let pet = harness
        .app
        .world_mut()
        .spawn((
            combat::hunter_pet::HunterPet { owner, slot: 0 },
            shared::components::Npc {
                template_id: 26125,
                name: "Ghoul".into(),
            },
            combat::maps::InMap::continent(0),
            shared::components::Position {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[owner]);
    let pet_reach = reaches.iter().find(|(unit, _, _)| *unit == pet).unwrap();
    assert_eq!(pet_reach.1, 8.0);
}

#[test]
fn lesser_ghoul_expiry_uses_the_pets_native_explosion_radius() {
    let mut harness = crate::test_support::harness(Vec::new(), &[], 6).unwrap();
    harness.app.insert_resource(
        crate::test_support::world_db()
            .unwrap()
            .spell_store()
            .clone(),
    );
    let owner = harness.caster;
    harness.app.world_mut().entity_mut(owner).insert(
        combat::class_progression::KnownSpells::from_baseline(vec![288848]),
    );
    let pet = harness
        .app
        .world_mut()
        .spawn((
            combat::maps::InMap::continent(0),
            combat::spell_cast::summon::SummonedCreature {
                owner,
                spell_id: 275430,
                properties_id: 0,
                properties: combat::spell_cast::summon::SummonProperties {
                    control: 0,
                    faction: 0,
                    title: 0,
                    slot: 0,
                    flags: 0,
                },
                display_scale: 1.0,
                expires_at: None,
            },
            shared::components::Position {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[owner]);
    let pet_reach = reaches.iter().find(|(unit, _, _)| *unit == pet).unwrap();
    assert_eq!(pet_reach.1, 8.0);
}

#[test]
fn army_ghoul_reports_its_owner_commanded_epidemic_order() {
    let mut harness = crate::test_support::harness(Vec::new(), &[], 6).unwrap();
    harness.app.insert_resource(
        crate::test_support::world_db()
            .unwrap()
            .spell_store()
            .clone(),
    );
    let owner = harness.caster;
    let pet = harness
        .app
        .world_mut()
        .spawn((
            combat::maps::InMap::continent(0),
            combat::spell_cast::summon::SummonedCreature {
                owner,
                spell_id: 1294026,
                properties_id: 0,
                properties: combat::spell_cast::summon::SummonProperties {
                    control: 0,
                    faction: 0,
                    title: 0,
                    slot: 0,
                    flags: 0,
                },
                display_scale: 1.0,
                expires_at: None,
            },
            shared::components::Position {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
        ))
        .id();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[owner]);
    let pet_reach = reaches.iter().find(|(unit, _, _)| *unit == pet).unwrap();
    assert_eq!(pet_reach.1, 8.0);
}

#[test]
fn beast_mastery_follower_without_area_actions_has_no_secondary_reach() {
    let (mut harness, unit) = follower(209065, 3);
    let reaches = party_unintended_reach(harness.app.world_mut(), &[unit]);
    assert_eq!((reaches[0].1, reaches[0].2), (0.0, 0.0));
}
