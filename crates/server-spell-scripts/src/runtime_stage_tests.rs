//! Observable launch snapshots and native/proc/supply ordering through the registry.
use super::*;
use combat::spell_cast::{
    script_registry::{self, SpellScriptHook},
    scripts,
};
use combat::spell_info::SpellInfo;

#[derive(Resource, Default)]
struct Calls(Vec<&'static str>);

fn native(world: &mut World, _: Entity, _: &SpellInfo) {
    world.resource_mut::<Calls>().0.push("native");
}
fn proc_cast(world: &mut World, _: Entity, _: &SpellInfo) {
    world.resource_mut::<Calls>().0.push("proc");
}
fn supply_cast(world: &mut World, _: Entity, _: &SpellInfo) {
    world.resource_mut::<Calls>().0.push("supply");
}
fn policy_cast(world: &mut World, _: Entity, _: &SpellInfo) {
    world.resource_mut::<Calls>().0.push("policy");
}
fn lookup_native(id: u32) -> Option<scripts::AfterCastHook> {
    (id == 133).then_some(native)
}
fn lookup_proc(id: u32) -> Option<scripts::AfterCastHook> {
    (id == 133).then_some(proc_cast)
}
fn lookup_supply(id: u32) -> Option<scripts::AfterCastHook> {
    (id == 133).then_some(supply_cast)
}
fn lookup_policy(id: u32) -> Option<scripts::AfterCastHook> {
    (id == 133).then_some(policy_cast)
}
fn snapshot(_: &World, _: Entity, spell: &mut SpellInfo) {
    spell.cast_launch_auras.push(190446);
}
fn child_scalar(_: &World, _: Entity, parent: &SpellInfo, child: u32) -> Option<f32> {
    (child == 228596 && parent.cast_launch_auras.contains(&190446)).then_some(1.75)
}
fn pending(world: &mut World) {
    world.resource_mut::<Calls>().0.push("pending");
}
fn launch(world: &mut World, _: Entity, spell: &SpellInfo) {
    assert_eq!(spell.cast_launch_auras, [190446]);
    world.resource_mut::<Calls>().0.push("launch");
}

#[test]
fn registry_retains_native_proc_supply_policy_order() {
    const HOOKS: &[SpellScriptHook] = &[
        SpellScriptHook::SupplyAfterCast(lookup_supply),
        SpellScriptHook::AfterPolicyCast(lookup_policy),
        SpellScriptHook::ProcAfterCast(lookup_proc),
        SpellScriptHook::NativeAfterCast(lookup_native),
    ];
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[HOOKS]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    let spell = SpellInfo {
        id: 133,
        ..Default::default()
    };
    for run in scripts::after_cast(&world, spell.id) {
        run(&mut world, caster, &spell);
    }
    assert_eq!(
        world.resource::<Calls>().0,
        ["native", "proc", "supply", "policy"]
    );
}

#[test]
fn registry_carries_launch_snapshot_into_child_and_delayed_stages() {
    const HOOKS: &[SpellScriptHook] = &[
        SpellScriptHook::SnapshotLaunchAuras(snapshot),
        SpellScriptHook::TriggeredChildDamageMultiplier(child_scalar),
        SpellScriptHook::AfterLaunchProcSupply(launch),
        SpellScriptHook::ResolvePendingScriptEvents(pending),
    ];
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[HOOKS]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    let mut spell = SpellInfo {
        id: 44614,
        ..Default::default()
    };
    assert_eq!(
        script_registry::triggered_child_damage_multiplier(&world, caster, &spell, 228596),
        None
    );
    script_registry::snapshot_launch_auras(&world, caster, &mut spell);
    assert_eq!(
        script_registry::triggered_child_damage_multiplier(&world, caster, &spell, 228596),
        Some(1.75)
    );
    assert_eq!(
        script_registry::triggered_child_damage_multiplier(&world, caster, &spell, 228354),
        None
    );
    script_registry::after_launch_proc_supply(&mut world, caster, &spell);
    script_registry::resolve_pending_script_events(&mut world);
    assert_eq!(world.resource::<Calls>().0, ["launch", "pending"]);
}
