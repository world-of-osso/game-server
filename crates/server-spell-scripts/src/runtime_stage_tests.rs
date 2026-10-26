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

fn requires_context(_: &World, _: Entity, spell: &SpellInfo) -> bool {
    spell.id == 32645
}

fn prepare_context(_: &World, _: Entity, spell: &mut SpellInfo) {
    spell.cast_damage_multiplier = Some(1.75);
}

fn context_first(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    target: Entity,
) {
    assert_eq!(target, ctx.caster);
    assert_eq!(ctx.depth, 3);
    assert_eq!(ctx.spell.cast_damage_multiplier, Some(1.75));
    world.resource_mut::<Calls>().0.push("context-first");
}

fn context_second(world: &mut World, _: &combat::spell_cast::effects::EffectCtx<'_>, _: Entity) {
    world.resource_mut::<Calls>().0.push("context-second");
}

fn first_enemy(spell: u32) -> Option<script_registry::BeforeEnemyContextHitHook> {
    (spell == 32645).then_some(context_first)
}

fn second_enemy(spell: u32) -> Option<script_registry::BeforeEnemyContextHitHook> {
    (spell == 32645).then_some(context_second)
}

#[test]
fn registry_keeps_policy_snapshot_and_pre_enemy_hit_stages_separate() {
    const ONE: &[SpellScriptHook] = &[
        SpellScriptHook::PolicyContextRequired(requires_context),
        SpellScriptHook::PreparePolicySpell(prepare_context),
        SpellScriptHook::BeforePolicyPrimaryHit(context_first),
        SpellScriptHook::BeforeEnemyContextHit(first_enemy),
    ];
    const TWO: &[SpellScriptHook] = &[
        SpellScriptHook::BeforePolicyPrimaryHit(context_second),
        SpellScriptHook::BeforeEnemyContextHit(second_enemy),
    ];
    let mut world = World::new();
    world.insert_resource(SpellScriptRegistry::new(&[ONE, TWO]));
    world.init_resource::<Calls>();
    let caster = world.spawn_empty().id();
    let mut spell = SpellInfo {
        id: 32645,
        ..Default::default()
    };
    assert!(script_registry::policy_context_required(
        &world, caster, &spell
    ));
    assert!(!script_registry::policy_context_required(
        &world,
        caster,
        &SpellInfo::default()
    ));
    script_registry::prepare_policy_spell(&world, caster, &mut spell);
    let ctx = combat::spell_cast::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 3,
        dest: None,
        chain_multiplier: 1.0,
    };
    script_registry::before_policy_primary_hit(&mut world, &ctx, caster);
    let run = script_registry::before_enemy_context_hit(&world, spell.id).unwrap();
    run(&mut world, &ctx, caster);
    assert!(script_registry::before_enemy_context_hit(&world, 133).is_none());
    assert_eq!(
        world.resource::<Calls>().0,
        ["context-first", "context-second", "context-first"]
    );
}

#[test]
fn registry_classifies_only_the_authored_aura_fields() {
    const FIRST: &[SpellScriptHook] = &[SpellScriptHook::NamedAuraFields(&[(41635, 0)])];
    const SECOND: &[SpellScriptHook] = &[SpellScriptHook::NamedAuraFields(&[(390994, 1)])];
    let registry = SpellScriptRegistry::new(&[FIRST, SECOND]);
    assert!(registry.consumes_named_aura_field(41635, 0));
    assert!(registry.consumes_named_aura_field(390994, 1));
    assert!(!registry.consumes_named_aura_field(390994, 0));
    assert!(!SpellScriptRegistry::new(&[]).consumes_named_aura_field(41635, 0));
}
