//! Approved follower pickup policy; native summoned pets, not pet-stat overrides.
use super::{Follower, ai};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::ThreatTable;

const GARRICK: u32 = 209057;
const HAND_OF_RECKONING: u32 = 62124;

fn prepare_tank(h: &mut Harness, pet_owner: Entity, pet: Entity) -> Entity {
    for unit in [h.caster, h.npc, pet_owner, pet] {
        crate::combat::combat_stop(h.app.world_mut(), unit);
        if let Some(mut threat) = h.app.world_mut().get_mut::<ThreatTable>(unit) {
            *threat = ThreatTable::default();
        }
    }
    let tank = super::native_fixture::spawn(h.app.world_mut(), h.caster, GARRICK)
        .expect("native tank profile/template/initializer");
    assert_eq!(
        h.app
            .world()
            .get::<Follower>(tank)
            .unwrap()
            .profile
            .taunt_spell,
        HAND_OF_RECKONING
    );
    tank
}

fn pet_only_enemy(h: &mut Harness, pet: Entity) -> Entity {
    let at = *h
        .app
        .world()
        .get::<shared::components::Position>(pet)
        .unwrap();
    let enemy = spawn_npc(&mut h.app, at.x);
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((at, ThreatTable::default()));
    crate::creature_aggro::attack_start(h.app.world_mut(), enemy, pet);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .unwrap()
            .0,
        Some(pet)
    );
    let entries = h.app.world().get::<ThreatTable>(enemy).unwrap().entries();
    assert_eq!(
        entries.len(),
        1,
        "native attack_start, not fake party threat"
    );
    assert_eq!(entries[0].entity, pet.to_bits());
    assert_eq!(entries[0].threat, 0.0);
    enemy
}

fn assert_native_pickup(h: &mut Harness, tank: Entity, pet_owner: Entity, pet: Entity) {
    let enemy = pet_only_enemy(h, pet);
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let party = ai::party_units(h.app.world_mut(), h.caster);
    assert!(party.contains(&pet_owner));
    assert!(!party.contains(&pet), "healer/party policy is unchanged");
    assert!(
        !crate::combat_refs::has_active_reference_to(h.app.world(), enemy, pet_owner),
        "observe pet-only acquisition BEFORE the first hit engages its owner"
    );
    let enemies = ai::engaged_enemies(h.app.world_mut(), &follower, tank, &party);
    assert_eq!(
        enemies.iter().map(|(enemy, _)| *enemy).collect::<Vec<_>>(),
        vec![enemy],
        "party-pet threat must be admitted without inventing owner threat"
    );
    for _ in 0..100 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        let taunted = h.inbox.go.iter().any(|event| {
            event.caster == tank.to_bits()
                && event.spell_id == HAND_OF_RECKONING
                && event.target == Some(enemy.to_bits())
        });
        let victim = h
            .app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .and_then(|target| target.0);
        if taunted && victim == Some(tank) {
            return;
        }
    }
    panic!("native Hand of Reckoning must pick up the admitted pet attacker");
}

#[test]
#[ignore = "RED: follower tank omits native hunter-pet-only combat"]
fn pet_coverage_native_duck_attacker_is_admitted_and_taunted() {
    let _seed = crate::random::seed_scope(1);
    let (mut h, owner, pet) = super::kit_hunter_policy_tests::prepare_native_duck(32);
    let tank = prepare_tank(&mut h, owner, pet);
    assert_native_pickup(&mut h, tank, owner, pet);
}

#[test]
#[ignore = "RED: follower tank omits native warlock-pet-only combat"]
fn pet_coverage_native_felguard_attacker_is_admitted_and_taunted() {
    let _seed = crate::random::seed_scope(1);
    let (mut fixture, pet) = super::pet_tests::felguard_at_level(32);
    let owner = fixture.unit;
    let tank = prepare_tank(&mut fixture.h, owner, pet);
    assert_native_pickup(&mut fixture.h, tank, owner, pet);
}

#[test]
fn pet_coverage_foreign_pet_and_unengaged_pack_are_not_recruited() {
    let _seed = crate::random::seed_scope(1);
    let (mut fixture, pet) = super::pet_tests::felguard_at_level(32);
    let owner = fixture.unit;
    let h = &mut fixture.h;
    let tank = prepare_tank(h, owner, pet);
    let stranger = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().get_mut::<Follower>(owner).unwrap().owner = stranger;
    let foreign_enemy = pet_only_enemy(h, pet);
    let fresh = spawn_npc(&mut h.app, 0.0);
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let party = ai::party_units(h.app.world_mut(), h.caster);
    assert!(!party.contains(&owner));
    let enemies = ai::engaged_enemies(h.app.world_mut(), &follower, tank, &party);
    assert!(
        !enemies
            .iter()
            .any(|(unit, _)| [foreign_enemy, fresh].contains(unit)),
        "foreign pet's fight and merely nearby pack must remain excluded"
    );
}
