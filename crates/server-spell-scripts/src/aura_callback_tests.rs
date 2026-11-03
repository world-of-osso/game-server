use crate::{combat, test_support};
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};

#[derive(Resource, Default)]
struct Calls(Vec<&'static str>);

fn first_apply(world: &mut World, _: Entity, _: &AuraInstance) {
    world.resource_mut::<Calls>().0.push("first apply");
}
fn second_apply(world: &mut World, _: Entity, _: &AuraInstance) {
    world.resource_mut::<Calls>().0.push("second apply");
}
fn first_remove(world: &mut World, _: Entity, _: &AuraInstance, _: RemoveReason) {
    world.resource_mut::<Calls>().0.push("first remove");
}
fn second_remove(world: &mut World, _: Entity, _: &AuraInstance, _: RemoveReason) {
    world.resource_mut::<Calls>().0.push("second remove");
}
fn first_periodic(_: &mut World, _: Entity, _: &AuraInstance, _: u8) {}
fn second_periodic(_: &mut World, _: Entity, _: &AuraInstance, _: u8) {}

fn apply_lunar_calling(world: &mut World, caster: Entity) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 429523,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn duplicate_aura_lifecycle_hooks_both_run_in_registration_order() {
    let mut harness = test_support::harness(vec![], &[429523], 11).expect("native spell data");
    let mut scripts = AuraScripts::default();
    scripts.0.insert(
        429523,
        AuraScript {
            on_apply: Some(first_apply),
            on_remove: Some(first_remove),
            ..default()
        },
    );
    scripts.0.entry(429523).or_default().on_apply = Some(second_apply);
    scripts.0.entry(429523).or_default().on_remove = Some(second_remove);
    // Another registration flushes the last entry mutation, as startup does.
    scripts.0.insert(1, AuraScript::default());
    let caster = harness.caster;
    let world = harness.app.world_mut();
    world.insert_resource(scripts);
    world.init_resource::<Calls>();
    apply_lunar_calling(world, caster);
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 429523
    });
    assert_eq!(
        world.resource::<Calls>().0,
        [
            "first apply",
            "second apply",
            "first remove",
            "second remove"
        ]
    );
}

#[test]
#[should_panic(expected = "duplicate aura hook on_periodic for spell 429523")]
fn duplicate_noncomposable_hook_fails_registration() {
    let mut scripts = AuraScripts::default();
    scripts.0.entry(429523).or_default().on_periodic = Some(first_periodic);
    scripts.0.entry(429523).or_default().on_periodic = Some(second_periodic);
    scripts.0.insert(1, AuraScript::default());
}

#[test]
fn balance_lunar_calling_still_starts_eclipse_override() {
    let mut harness =
        test_support::harness(vec![], &[1239669, 429523, 1233353], 11).expect("native spell data");
    let caster = harness.caster;
    let world = harness.app.world_mut();
    world
        .entity_mut(caster)
        .insert(shared::components::ActiveSpec(102));
    apply_lunar_calling(world, caster);
    assert!(
        auras::has_aura(world, caster, 1233353),
        "Balance Lunar Calling must apply its Eclipse override"
    );
}
