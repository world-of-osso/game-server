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

fn select_caster(
    _: &mut World,
    caster: Entity,
    _: &SpellInfo,
    _: Option<Entity>,
) -> combat::spell_cast::TargetPlan {
    vec![combat::spell_cast::PlannedEffect {
        index: 0,
        units: vec![caster],
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn plan_first(
    spell: u32,
) -> Option<(
    combat::spell_cast::targets::Relation,
    script_registry::TargetPlanHook,
)> {
    (spell == 133).then_some((combat::spell_cast::targets::Relation::Ally, select_caster))
}

fn plan_second(
    spell: u32,
) -> Option<(
    combat::spell_cast::targets::Relation,
    script_registry::TargetPlanHook,
)> {
    (spell == 133).then_some((combat::spell_cast::targets::Relation::Enemy, select_caster))
}

fn tick_first(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    amount: f32,
) {
    assert_eq!(ctx.depth, 3);
    assert_eq!(amount, 42.0);
    first(world, target, ctx.spell);
}

fn tick_second(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    amount: f32,
) {
    assert_eq!(ctx.depth, 3);
    assert_eq!(amount, 42.0);
    second(world, target, ctx.spell);
}

fn factor_two(_: &World, _: Entity, _: Entity, _: &SpellInfo) -> f32 {
    2.0
}
fn factor_three(_: &World, _: Entity, _: Entity, _: &SpellInfo) -> f32 {
    3.0
}

#[test]
fn registry_preserves_target_selection_and_damage_stage_arguments() {
    const ONE: &[SpellScriptHook] = &[
        SpellScriptHook::AfterResourceValues(lookup_first),
        SpellScriptHook::TargetPlan(plan_first),
        SpellScriptHook::PeriodicPrimaryDamage(tick_first),
        SpellScriptHook::TargetDamageMultiplier(factor_two),
    ];
    const TWO: &[SpellScriptHook] = &[
        SpellScriptHook::AfterResourceValues(lookup_second),
        SpellScriptHook::TargetPlan(plan_second),
        SpellScriptHook::PeriodicPrimaryDamage(tick_second),
        SpellScriptHook::TargetDamageMultiplier(factor_three),
    ];
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[ONE, TWO]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    let target = world.spawn_empty().id();
    let spell = SpellInfo {
        id: 133,
        ..Default::default()
    };
    let (relation, select) = script_registry::target_plan(&world, 133).unwrap();
    assert!(matches!(
        relation,
        combat::spell_cast::targets::Relation::Ally
    ));
    assert_eq!(
        select(&mut world, caster, &spell, Some(target))[0].units,
        vec![caster]
    );
    assert!(script_registry::target_plan(&world, 116).is_none());
    for hook in script_registry::after_resource_values(&world, 133) {
        hook(&mut world, caster, &spell);
    }
    let ctx = combat::spell_cast::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 3,
        dest: None,
        chain_multiplier: 1.0,
    };
    script_registry::periodic_primary_damage(&mut world, &ctx, target, 42.0);
    assert_eq!(
        world.resource::<Calls>().0,
        [
            ("first", caster, 133),
            ("second", caster, 133),
            ("first", target, 133),
            ("second", target, 133),
        ]
    );
    assert_eq!(
        script_registry::target_damage_multiplier(&world, caster, target, &spell),
        6.0
    );
    world.insert_resource(SpellScriptRegistry::new(&[]));
    assert!(script_registry::target_plan(&world, 133).is_none());
    assert_eq!(
        script_registry::after_resource_values(&world, 133).count(),
        0
    );
    script_registry::periodic_primary_damage(&mut world, &ctx, target, 42.0);
    assert_eq!(world.resource::<Calls>().0.len(), 4);
    assert_eq!(
        script_registry::target_damage_multiplier(&world, caster, target, &spell),
        1.0
    );
}
