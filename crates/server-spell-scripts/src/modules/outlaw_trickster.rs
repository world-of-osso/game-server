//! Earned Trickster strikes and their native aura/counter lifecycle.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::random::Rng;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 441146;
const UNSEEN: u32 = 441144;
const QUIET: u32 = 459485;
const FLASHING_STEEL: u32 = 1276630;
const CLEVER_COMBATANT: u32 = 1276679;
const COUP: u32 = 441423;
const ESCALATING: u32 = 441786;
const FLAWLESS_TALENT: u32 = 441321;
const FLAWLESS: u32 = 441326;
const DISORIENTING: u32 = 441274;
const FAZED: u32 = 441224;
const DEVIOUS: u32 = 441263;

// sc_rogue.cpp11102–11110 explicitly has no separate runtime buff spell.
// The selected native441274 passive remains applied for its cooldown modifiers;
// these uses are earned separately by actual Spree execution, never selection.
#[derive(Component)]
struct DisorientingUses(u8);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::BeforeNativeOnCast(prepare_spree),
    SpellScriptHook::AfterNativeOnCast(after_unseen_execute),
    SpellScriptHook::SpellResidual(after_builder_residual),
    SpellScriptHook::ResolvedDamage(after_hit),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        441346,
        AuraScript {
            on_proc: Some(reset_unseen_cooldown),
            ..Default::default()
        },
    );
}

// SimC76f8118 sc_rogue.cpp11539–11564: the selected native melee RPPM
// proc expires Unseen Blade's quiet cooldown, without bypassing its native ICD.
fn reset_unseen_cooldown(world: &mut World, caster: Entity, _: &AuraInstance, _: Option<Entity>) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == QUIET && aura.caster == caster
    });
}

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn after_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if damage <= 0.0 {
        return;
    }
    match ctx.spell.id {
        UNSEEN => finish_unseen_hit(world, ctx, target),
        57841 if auras::has_aura(world, ctx.caster, DEVIOUS) => apply_fazed(world, ctx, target),
        _ => {}
    }
}

fn after_builder_residual(world: &mut World, ctx: &EffectCtx, target: Entity, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    // sc_rogue.cpp2832–2837/5986–5991: the builder's Blade Flurry
    // resolves before its Unseen Blade can earn Flawless Form.
    match ctx.spell.id {
        193315 | 8676 => trigger_unseen(world, ctx, target),
        185763 if auras::has_aura(world, ctx.caster, CLEVER_COMBATANT) => {
            trigger_unseen(world, ctx, target)
        }
        _ => {}
    }
}

fn read_talent_instance(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == TALENT && aura.caster == caster)
        .map(|aura| aura.instance_id)
}

fn has_disorienting_use(world: &World, caster: Entity) -> bool {
    world
        .get::<DisorientingUses>(caster)
        .is_some_and(|uses| uses.0 > 0)
}

fn trigger_unseen(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let Some(instance) = read_talent_instance(world, ctx.caster) else {
        return;
    };
    let disorienting = has_disorienting_use(world, ctx.caster);
    if auras::has_aura(world, ctx.caster, QUIET) && !disorienting {
        return;
    }
    // sc_rogue.cpp8821–8850: even earned uses cannot bypass the native ICD.
    if auras::procs::start_policy_proc(world, ctx.caster, instance).is_none() {
        return;
    }
    spell_cast::cast_triggered(world, ctx.caster, UNSEEN, Some(target), ctx.depth + 1);
    roll_flashing_steel(world, ctx, target);
    if disorienting {
        world.get_mut::<DisorientingUses>(ctx.caster).unwrap().0 -= 1;
    } else {
        spell_cast::cast_triggered(world, ctx.caster, QUIET, Some(ctx.caster), ctx.depth + 1);
    }
}

fn roll_flashing_steel(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !auras::has_aura(world, ctx.caster, FLASHING_STEEL) {
        return;
    }
    let chance = script_effect_value(world, ctx.caster, FLASHING_STEEL, 1);
    if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
        spell_cast::cast_triggered(world, ctx.caster, UNSEEN, Some(target), ctx.depth + 1);
    }
}

fn after_unseen_execute(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    // sc_rogue.cpp6697–6700: one actual Unseen execution earns one stack,
    // including Flashing Steel's separate strike, even if that strike misses.
    if spell == UNSEEN && auras::has_aura(world, caster, COUP) {
        spell_cast::cast_triggered(world, caster, ESCALATING, Some(caster), depth + 1);
    }
}

fn finish_unseen_hit(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // sc_rogue.cpp6702–6711: actual impact earns owned Fazed and Flawless.
    apply_fazed(world, ctx, target);
    if auras::has_aura(world, ctx.caster, FLAWLESS_TALENT) {
        spell_cast::cast_triggered(world, ctx.caster, FLAWLESS, Some(ctx.caster), ctx.depth + 1);
    }
}

fn apply_fazed(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // Cloud Cover already applied this hit through its per-target cooldown.
    if !auras::has_aura(world, ctx.caster, 441640) {
        spell_cast::cast_triggered(world, ctx.caster, FAZED, Some(target), ctx.depth + 1);
    }
}

fn prepare_spree(
    world: &mut World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
    depth: u8,
) {
    if spell.id != 51690 || depth != 0 {
        return;
    }
    // sc_rogue.cpp4812–4821: real Spree grants these before its tick-zero
    // event; talent selection alone cannot grant Disorienting runtime uses.
    if auras::has_aura(world, caster, FLAWLESS_TALENT) {
        spell_cast::cast_triggered(world, caster, FLAWLESS, Some(caster), 1);
    }
    if auras::has_aura(world, caster, DISORIENTING) {
        let count = script_effect_value(world, caster, DISORIENTING, 1) as u8;
        world.entity_mut(caster).insert(DisorientingUses(count));
    }
}
