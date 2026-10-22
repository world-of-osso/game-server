//! HUNTER_WILD_THRASH_PET_CHILD_SIMC: qualify real pet geometry before parent RED.
use super::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;

const WILD_THRASH_PET: u32 = 1264355;
const WILD_THRASH_OWNER: u32 = 1264359;

fn distance(world: &World, from: Entity, to: Entity) -> f32 {
    let from = world.get::<Position>(from).unwrap();
    let to = world.get::<Position>(to).unwrap();
    Vec3::new(from.x, from.y, from.z).distance(Vec3::new(to.x, to.y, to.z))
}

fn approach_dummy_with_pet(run: &mut Run, pet: Entity) -> f32 {
    let target = run.targets[0];
    let radius = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WILD_THRASH_PET)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .radius[1]
        .unwrap()
        .radius;
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(target));
    for _ in 0..500 {
        if distance(run.app.world(), pet, target) < radius {
            return radius;
        }
        run.app.update();
    }
    panic!(
        "native pet never approached child area: distance={} radius={radius}",
        distance(run.app.world(), pet, target)
    );
}

fn pet_child_damage(world: &World, pet: Entity, target: Entity) -> i64 {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(pet.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(WILD_THRASH_PET)
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "direct native pet-child qualification; requires prepared parity inputs"]
fn dps_parity_wild_thrash_direct_pet_child_has_native_damage_and_geometry() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let target = run.targets[0];
    let radius = approach_dummy_with_pet(&mut run, pet);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    // Child is a background pet action; ordinary parent forwarding uses this
    // same triggered API. Real hit outcomes remain enabled; no RNG/stat overrides.
    for _ in 0..16 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            pet,
            WILD_THRASH_PET,
            Some(target),
            1,
        );
        run.app.update();
        if pet_child_damage(run.app.world(), pet, target) > 0 {
            break;
        }
    }
    let world = run.app.world();
    let damage = pet_child_damage(world, pet, target);
    assert!(
        damage > 0,
        "direct native1264355 must deal actual attributed pet damage"
    );
    assert!(distance(world, pet, target) < radius);
    println!(
        "WILD_THRASH_DIRECT pet={} target={} gap={} native_radius={radius} damage={damage}",
        pet.to_bits(),
        target.to_bits(),
        distance(world, pet, target)
    );
}

#[test]
#[ignore = "native Wild Thrash parent RED; requires prepared parity inputs"]
fn dps_parity_wild_thrash_parent_commands_actual_owned_pet_damage() {
    let (mut run, pet) = super::power_tests::prepare_hunter_and_pet();
    let target = run.targets[0];
    let radius = approach_dummy_with_pet(&mut run, pet);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        WILD_THRASH_OWNER,
        Some(target),
        false,
    )
    .expect("actual native owner root must be admitted");
    for _ in 0..40 {
        run.app.update();
    }
    let world = run.app.world();
    assert!(
        world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess
                && event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(WILD_THRASH_OWNER)),
        "actual owner root must finish before child assertion"
    );
    assert!(distance(world, pet, target) < radius);
    let damage = pet_child_damage(world, pet, target);
    assert!(
        damage > 0,
        "HUNTER_WILD_THRASH_PET_CHILD_SIMC: real parent must dispatch qualified native child from actual owned pet"
    );
}
