use super::*;
use combat::spell_cast::{
    CostPool,
    script_registry::{self, SpellScriptHook},
};
use combat::spell_info::SpellInfo;

#[derive(Resource, Default)]
struct Calls(Vec<(&'static str, Entity, u32)>);

fn first(world: &mut World, caster: Entity, spell: &SpellInfo) {
    world
        .resource_mut::<Calls>()
        .0
        .push(("first", caster, spell.id));
}
fn second(world: &mut World, caster: Entity, spell: &SpellInfo) {
    world
        .resource_mut::<Calls>()
        .0
        .push(("second", caster, spell.id));
}
fn lookup_first(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    (spell == 133).then_some(first)
}
fn lookup_second(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    (spell == 133).then_some(second)
}
fn record_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    assert_eq!(depth, 3);
    world
        .resource_mut::<Calls>()
        .0
        .push(("cast", caster, spell));
}
fn record_payment(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    assert_eq!(costs, &[(CostPool::Health, 42.0)]);
    world.resource_mut::<Calls>().0.push(("payment", caster, 0));
}

const FIRST: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(lookup_first),
    SpellScriptHook::AuraRollAfterCast(lookup_first),
    SpellScriptHook::OnCast(record_cast),
    SpellScriptHook::AfterPayment(record_payment),
];
const SECOND: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(lookup_second),
    SpellScriptHook::AuraRollAfterCast(lookup_second),
    SpellScriptHook::OnCast(record_cast),
    SpellScriptHook::AfterPayment(record_payment),
];

#[test]
fn registry_keeps_first_match_and_additive_module_order() {
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[FIRST, &[], SECOND]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    let spell = SpellInfo {
        id: 133,
        ..Default::default()
    };
    script_registry::native_after_cast(&world, spell.id).unwrap()(&mut world, caster, &spell);
    for hook in script_registry::aura_roll_after_cast(&world, spell.id) {
        hook(&mut world, caster, &spell);
    }
    script_registry::on_cast(&mut world, caster, spell.id, 3);
    script_registry::after_payment(&mut world, caster, &[(CostPool::Health, 42.0)]);
    assert_eq!(
        world.resource::<Calls>().0,
        [
            ("first", caster, 133),
            ("first", caster, 133),
            ("second", caster, 133),
            ("cast", caster, 133),
            ("cast", caster, 133),
            ("payment", caster, 0),
            ("payment", caster, 0),
        ]
    );
    assert!(script_registry::native_after_cast(&world, 116).is_none());
    assert_eq!(
        script_registry::aura_roll_after_cast(&world, 116).count(),
        0
    );
}

#[test]
fn empty_registry_has_no_script_side_effects() {
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    script_registry::on_cast(&mut world, caster, 133, 3);
    script_registry::after_payment(&mut world, caster, &[(CostPool::Health, 42.0)]);
    assert!(script_registry::native_after_cast(&world, 133).is_none());
    assert_eq!(
        script_registry::aura_roll_after_cast(&world, 133).count(),
        0
    );
    assert!(world.resource::<Calls>().0.is_empty());
}

#[test]
fn plugin_installs_real_modules_before_or_after_combat_initialization() {
    for scripts_first in [false, true] {
        let mut app = App::new();
        if scripts_first {
            app.add_plugins(SpellScriptsPlugin);
        }
        app.init_resource::<SpellScriptRegistry>();
        if !scripts_first {
            app.add_plugins(SpellScriptsPlugin);
        }
        assert!(script_registry::native_after_cast(app.world(), 383328).is_some());
        assert!(script_registry::native_after_cast(app.world(), 871).is_some());
        assert_eq!(
            script_registry::aura_roll_after_cast(app.world(), 1214909).count(),
            1
        );
        assert!(script_registry::native_after_cast(app.world(), 133).is_none());
    }
}
