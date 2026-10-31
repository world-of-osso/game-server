//! SimC76f8118 sc_rogue.cpp6444–6463/9011–9028: a landed finisher
//! copies its native residual across the caster's active Goremaw DoTs.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
    targets,
};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const TALENT: u32 = 426591;
const DOT: u32 = 426593;
const FRACTION: u32 = 1309274;
const RESIDUAL: u32 = 1309309;
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::ResolvedDamage(after_damage),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    _spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if has_talent(world, caster, TALENT) {
        combat::spell_cast::script_registry::ScriptReach::EngagedOnly
    } else {
        Default::default()
    }
}

fn after_damage(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    if damage <= 0.0 || !has_talent(world, ctx.caster, TALENT) {
        return;
    }
    let points = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .and_then(|state| {
            state
                .resources
                .iter()
                .find(|(power, _)| *power == PowerType::ComboPoints)
        })
        .map_or(0.0, |(_, points)| *points);
    // These native payloads are dispatched only by a paid finisher; their
    // effective points are already baked into the child damage snapshot.
    let finisher_child = matches!(
        ctx.spell.id,
        328082 | 319190 | 1269205 | 1269565 | 280720 | 282449 | 1269560
    );
    if points <= 0.0 && !finisher_child {
        return;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(RESIDUAL)
        .expect("native Goremaw residual")
        .clone();
    let radius = spell.effects[0].radius[1]
        .expect("native Goremaw radius")
        .radius;
    let center = targets::position(world, ctx.caster).expect("Goremaw caster position");
    let recipients: Vec<_> = targets::area_units(
        world,
        ctx.caster,
        center,
        radius,
        targets::Relation::Enemy,
        0,
    )
    .into_iter()
    .filter(|target| {
        world.get::<ActiveAuras>(*target).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == DOT && aura.caster == ctx.caster)
        })
    })
    .collect();
    if recipients.is_empty() {
        return;
    }
    let fraction = script_effect_value(world, ctx.caster, FRACTION, 0) / 100.0;
    let amount = damage
        * fraction
        * super::subtlety_shadow::shadow_multiplier(world, ctx)
        * super::subtlety_shadow::darkness_multiplier(world, ctx.caster)
        / recipients.len() as f32;
    let copy = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for target in recipients {
        effects::deal_copied_damage(world, &copy, target, amount);
    }
}
