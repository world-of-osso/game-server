//! Source-qualified native Void Ray channel children.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
    targets,
};
use combat::spell_info::SpellStore;
use shared::{
    casting::{CastState, CastType},
    components::PowerType,
};

const VOID_RAY: u32 = 473728;
const META: u32 = 1217607;
const FOCUSED_RAY: u32 = 1240203;

#[derive(Component)]
struct VoidRayTick(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OnCast(snapshot_tick),
    SpellScriptHook::ChannelPeriodic(periodic_trigger),
    SpellScriptHook::TargetContextDamageMultiplier(focused_ray),
    SpellScriptHook::AfterFireDirectHit(shatter_on_tick),
];

fn snapshot_tick(world: &mut World, actor: Entity, spell: u32, _: u8) {
    if spell != VOID_RAY {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp6354–6367,6420–6427 selects
    // the native child at channel start, not again at each tick.
    let child = if combat::auras::has_aura(world, actor, META) {
        // SimC76f8118 cpp6368–6372,6419–6427: Meta's effect8 plus selected
        // Voidpurge effect1 supplies the hasted cooldown absent from the base spell.
        let mut cooldown = spell_cast::scripts::script_effect_value(world, actor, META, 7);
        if world
            .get::<KnownSpells>(actor)
            .is_some_and(|known| known.contains(1244220))
        {
            cooldown += spell_cast::scripts::script_effect_value(world, actor, 1244220, 0);
        }
        spell_cast::scripts::start_hasted_cooldown(world, actor, VOID_RAY, cooldown);
        1214595
    } else {
        1213649
    };
    world.entity_mut(actor).insert(VoidRayTick(child));
}

fn focused_ray(world: &World, ctx: &EffectCtx<'_>, _: Entity, periodic: bool) -> f32 {
    if periodic || !matches!(ctx.spell.id, 1213649 | 1214595) {
        return 1.0;
    }
    if !world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|known| known.contains(FOCUSED_RAY))
    {
        return 1.0;
    }
    // SimC76f8118 sc_demon_hunter.cpp6304–6321: the native talent applies
    // its effect1 percentage only through its effect2 resolved-target threshold.
    let targets = spell_cast::resolving_targets(world, ctx)
        .expect("native Void Ray tick retains resolved targets")
        .iter()
        .filter(|&&target| {
            targets::relation_matches(world, ctx.caster, target, targets::Relation::Enemy)
        })
        .count();
    let limit = spell_cast::scripts::script_effect_value(world, ctx.caster, FOCUSED_RAY, 1);
    if targets == 0 || targets as f32 > limit {
        return 1.0;
    }
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, FOCUSED_RAY, 0);
    1.0 + percent / 100.0
}

fn shatter_on_tick(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool) {
    if !matches!(ctx.spell.id, 1213649 | 1214595)
        || !world
            .get::<KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(1227619))
    {
        return;
    }
    let targets = spell_cast::resolving_targets(world, ctx)
        .expect("native Void Ray tick retains resolved targets")
        .iter()
        .filter(|&&target| {
            targets::relation_matches(world, ctx.caster, target, targets::Relation::Enemy)
        })
        .count();
    assert!(targets > 0, "Void Ray hit requires a target");
    // SimC76f8118 cpp3390–3437,6338–6344: base chance +1%, Waste Not, divided by target count.
    let mut chance = spell_cast::scripts::script_effect_value(world, ctx.caster, 1227619, 0) + 1.0;
    if world
        .get::<KnownSpells>(ctx.caster)
        .is_some_and(|known| known.contains(1223918))
    {
        chance *=
            1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, 1223918, 0) / 100.0;
    }
    if combat::random::thread_rng().gen_range(0.0..100.0) < chance / targets as f32 {
        soul_fragments::spawn(world, ctx.caster, FragmentKind::Lesser, false)
            .expect("native Shattered Souls fragment");
    }
}

fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (spell == VOID_RAY && index == 0).then_some(cast_tick)
}

fn cast_tick(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let actor = ctx.caster;
    if !world
        .get::<CastState>(actor)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == VOID_RAY)
    {
        return;
    }
    let Some(child) = world.get::<VoidRayTick>(actor).map(|tick| tick.0) else {
        error!(?actor, "Void Ray channel missing its selected tick");
        return;
    };
    if child == 1213649 && !pay_tick_cost(world, actor, child) {
        spell_cast::cancel_cast(world, actor);
        return;
    }
    spell_cast::cast_triggered(world, actor, child, None, 1);
}

fn pay_tick_cost(world: &mut World, actor: Entity, child: u32) -> bool {
    // SimC76f8118 sc_demon_hunter.cpp6431–6455 pays the child cost per tick.
    let cost: f32 = world
        .resource::<SpellStore>()
        .get(child)
        .expect("native Void Ray child")
        .power_costs
        .iter()
        .filter(|cost| {
            cost.power_type == PowerType::Fury as i32 && cost.required_aura_spell_id == 0
        })
        .map(|cost| cost.mana_cost as f32)
        .sum();
    assert!(cost > 0.0, "native Void Ray child requires a Fury cost");
    let current = spell_cast::powers::power_value(world, actor, PowerType::Fury)
        .expect("Void Ray caster requires Fury")
        .0;
    if current < cost {
        return false;
    }
    spell_cast::powers::change_power(world, actor, PowerType::Fury, -cost)
        .expect("Void Ray caster requires Fury");
    true
}
