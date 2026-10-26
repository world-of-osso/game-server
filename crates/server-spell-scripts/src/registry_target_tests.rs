use super::*;
use combat::spell_cast::{
    effects::{EffectCtx, EffectHandler},
    script_registry::SpellScriptHook,
};
use combat::spell_info::{SpellEffect, SpellInfo};

#[derive(Resource, Default)]
struct Hits(Vec<(Entity, Entity, u32)>);
fn first(world: &mut World, ctx: &EffectCtx, _: &SpellEffect, target: Entity) {
    world
        .resource_mut::<Hits>()
        .0
        .push((ctx.caster, target, ctx.spell.id));
}
fn second(_: &mut World, _: &EffectCtx, _: &SpellEffect, _: Entity) {
    panic!("second matching handler must not run");
}
fn lookup_first(spell: u32, effect: u8) -> Option<EffectHandler> {
    ((spell, effect) == (133, 1)).then_some(first)
}
fn lookup_second(spell: u32, effect: u8) -> Option<EffectHandler> {
    ((spell, effect) == (133, 1)).then_some(second)
}
fn exclude_primary(
    _: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    assert_eq!((spell.id, effect), (133, 1));
    assert_ne!(Some(caster), primary);
    units.retain(|unit| Some(*unit) != primary);
}
fn remove_last(
    _: &World,
    _: Entity,
    _: &SpellInfo,
    _: u8,
    _: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    units.pop();
}
const FIRST: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(lookup_first),
    SpellScriptHook::TargetFilter(exclude_primary),
];
const SECOND: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(lookup_second),
    SpellScriptHook::TargetFilter(remove_last),
];

#[test]
fn registry_effect_handler_and_target_filters_preserve_context_and_order() {
    let mut world = World::new();
    world.init_resource::<Hits>();
    let caster = world.spawn_empty().id();
    let primary = world.spawn_empty().id();
    let other = world.spawn_empty().id();
    let last = world.spawn_empty().id();
    let registry = SpellScriptRegistry::new(&[FIRST, &[], SECOND]);
    let spell = SpellInfo {
        id: 133,
        ..Default::default()
    };
    let mut units = vec![other, primary, last];
    registry.filter_targets(&world, caster, &spell, 1, Some(primary), &mut units);
    assert_eq!(units, vec![other]);
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    registry.effect_handler(133, 1).unwrap()(&mut world, &ctx, &SpellEffect::default(), units[0]);
    assert_eq!(world.resource::<Hits>().0, vec![(caster, other, 133)]);
    assert!(registry.effect_handler(133, 0).is_none());
}
