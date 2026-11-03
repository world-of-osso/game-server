//! MID2 Feral two-piece banks landed finisher points until Berserk ends.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const SET_BONUS: u32 = 1296605;
const BERSERK: u32 = 106951;
const INCARNATION: u32 = 102543;
const ASHAMANES_GUIDANCE: u32 = 391548;
const FURY: u32 = 1301600;
const DRUID_FAMILY: u32 = 7;

#[derive(Component, Default)]
struct BankedFury {
    duration_ms: u32,
    next_at: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::BeforePolicyPrimaryHit(bank_points),
];

fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    auras::has_aura(world, caster, SET_BONUS)
        && spell
            .class_options
            .as_ref()
            .is_some_and(|options| options.spell_class_set == DRUID_FAMILY)
        && spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
}

fn bank_points(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let Some(context) = ctx.spell.cast_policy_context.as_ref() else {
        return;
    };
    if (ctx.depth != 0 && !context.free_spender) || !needs_context(world, ctx.caster, ctx.spell) {
        return;
    }
    let incarnation = auras::has_aura(world, ctx.caster, INCARNATION);
    if !incarnation && !auras::has_aura(world, ctx.caster, BERSERK) {
        return;
    }
    let Some(points) = context
        .resources
        .iter()
        .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
    else {
        return;
    };
    let index = u8::from(incarnation && has_talent(world, ctx.caster, ASHAMANES_GUIDANCE));
    let per_point = spell_cast::scripts::script_effect_value(world, ctx.caster, SET_BONUS, index);
    let now = spell_cast::now(world);
    let cooldown = world
        .resource::<combat::spell_info::SpellStore>()
        .get(SET_BONUS)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Halazzi set proc metadata")
        .proc_category_recovery_ms as f64
        / 1000.0;
    let mut entity = world.entity_mut(ctx.caster);
    let mut bank = entity.entry::<BankedFury>().or_default().into_mut();
    if now >= bank.next_at {
        bank.duration_ms += (per_point * points).round() as u32;
        bank.next_at = now + cooldown;
    }
}

pub(super) fn finish_berserk(world: &mut World, caster: Entity) {
    let duration = world
        .get::<BankedFury>(caster)
        .map_or(0, |bank| bank.duration_ms);
    world.entity_mut(caster).remove::<BankedFury>();
    if duration == 0 {
        return;
    }
    // VERIFIED_WCL g1dGBjmAtcHYXaM9 fight15 actor162: both Berserks
    // grant 75s, equal to landed paid and free effective combo points.
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        FURY,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            fixed_aura_duration_ms: Some(duration),
            ..Default::default()
        },
    );
}
