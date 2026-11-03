//! A persistent Hunter follower keeps its precombat pet across pulls.
use super::follower_dps_diagnostic::*;
use super::*;
use server_follower_apl::{FollowerApl, FollowerAplFailed};
use shared::protocol::{CombatLogEvent, CombatLogKind};

const CALL_PET: u32 = 883;

/// One frame; the APL must survive it, never fail, and the same pet must stay out.
fn step(app: &mut App, actor: Entity, pet: Entity) -> bool {
    app.update();
    let world = app.world();
    assert!(world.get::<FollowerAplFailed>(actor).is_none());
    assert_eq!(crate::hunter_pet::summoned_pet(world, actor), Some(pet));
    assert!(crate::spell_cast::targets::is_alive(world, pet));
    world
        .get::<FollowerApl>(actor)
        .expect("follower APL persists between frames")
        .in_combat()
}

fn engage(app: &mut App, owner: Entity, target: Entity) {
    app.world_mut().entity_mut(owner).insert((
        crate::networking::CombatTarget(Some(target)),
        crate::combat::AutoAttack::new(target),
    ));
    crate::combat_refs::set_in_combat_with(app.world_mut(), owner, target);
}

fn kill(app: &mut App, target: Entity) {
    app.world_mut()
        .entity_mut(target)
        .insert(shared::components::Health {
            current: 0.0,
            max: 1.0,
        });
}

fn count_events(app: &App, source: Entity, matches: impl Fn(&CombatLogEvent) -> bool) -> usize {
    app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.source == Some(source.to_bits()) && matches(event))
        .count()
}

fn assert_pet_kept_across_three_pulls(spec: u32) {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, spare, _) = prepare_class_follower(app, copy, spec, 90);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    kill(&mut app, spare);
    app.update();
    let pet = crate::hunter_pet::summoned_pet(app.world(), actor)
        .expect("precombat summon_pet calls the native pet");
    for _ in 0..200 {
        assert!(!step(&mut app, actor, pet), "no pull yet");
    }
    for pull in 1..=3 {
        let target = spawn_level_dummy(&mut app, actor, 90, pull).unwrap();
        engage(&mut app, owner, target);
        let combat_frames = (0..400).filter(|_| step(&mut app, actor, pet)).count();
        assert!(
            combat_frames > 300,
            "pull {pull} in combat {combat_frames}/400 frames"
        );
        let pet_hits = count_events(&app, pet, |event| {
            event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        });
        assert!(pet_hits > 0, "the kept pet fights pull {pull}");
        kill(&mut app, target);
        for _ in 0..100 {
            step(&mut app, actor, pet);
        }
        assert!(
            !step(&mut app, actor, pet),
            "pull {pull} ends out of combat"
        );
    }
    assert_eq!(
        app.world()
            .get::<crate::hunter_pet::HunterPet>(pet)
            .map(|pet| pet.owner),
        Some(actor)
    );
    let summons = count_events(&app, actor, |event| {
        event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(CALL_PET)
    });
    assert_eq!(summons, 1, "the pet is summoned once, never re-summoned");
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn beast_mastery_follower_keeps_its_pet_across_three_pulls() {
    assert_pet_kept_across_three_pulls(253);
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn survival_follower_keeps_its_pet_across_three_pulls() {
    assert_pet_kept_across_three_pulls(255);
}
