//! Secondary residual attacks copy the landed source before recipient modifiers.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    targets,
};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SpellResidual(after_spell),
    SpellScriptHook::MeleeResidual(after_melee),
];

struct SecondaryCopy {
    aura: u32,
    fraction_effect: u8,
    target_effect: u8,
    payload: u32,
    radius: f32,
    spells: &'static [u32],
    white_hits: bool,
    extra_targets: (u32, u8),
    bonus_talent: u32,
    bonus_aura: (u32, u8),
}

// SimC76f8118 sc_rogue.cpp2068–2105,3884–3905,8061–8087.
const COPIES: &[SecondaryCopy] = &[SecondaryCopy {
    aura: 13877,
    fraction_effect: 1,
    target_effect: 2,
    payload: 22482,
    radius: 5.0,
    spells: &[
        8676, 2098, 315341, 57841, 57842, 185763, 86392, 1265918, 193315, 197834, 452538, 441144,
        462140, 462239, 462240,
    ],
    white_hits: true,
    extra_targets: (272026, 2),
    bonus_talent: 441367,
    bonus_aura: (441326, 2),
}];

struct CopySource {
    caster: Entity,
    primary: Entity,
    spell: Option<u32>,
    amount: f32,
    depth: u8,
}

fn after_spell(world: &mut World, ctx: &EffectCtx, primary: Entity, amount: f32) {
    copy_secondary(
        world,
        CopySource {
            caster: ctx.caster,
            primary,
            spell: Some(ctx.spell.id),
            amount,
            depth: ctx.depth + 1,
        },
    );
}

pub(crate) fn after_melee(world: &mut World, caster: Entity, primary: Entity, amount: f32) {
    copy_secondary(
        world,
        CopySource {
            caster,
            primary,
            spell: None,
            amount,
            depth: 1,
        },
    );
}

fn read_aura_amount(world: &World, caster: Entity, spell: u32, index: u8) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)?;
    Some(
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native secondary-copy aura effect")
            .amount,
    )
}

fn read_copy_parameters(
    world: &World,
    source: &CopySource,
    policy: &SecondaryCopy,
) -> Option<(f32, usize)> {
    let fraction =
        read_aura_amount(world, source.caster, policy.aura, policy.fraction_effect)? / 100.0;
    let mut cap = script_effect_value(world, source.caster, policy.aura, policy.target_effect);
    let (extra_aura, extra_effect) = policy.extra_targets;
    if combat::auras::has_aura(world, source.caster, extra_aura) {
        cap += script_effect_value(world, source.caster, extra_aura, extra_effect);
    }
    let (bonus_aura, bonus_effect) = policy.bonus_aura;
    let bonus = if combat::auras::has_aura(world, source.caster, policy.bonus_talent) {
        read_aura_amount(world, source.caster, bonus_aura, bonus_effect)
            .map_or(1.0, |amount| 1.0 + amount / 100.0)
    } else {
        1.0
    };
    assert!(
        cap > 0.0,
        "native secondary-copy target cap must be positive"
    );
    Some((source.amount * fraction * bonus, cap as usize))
}

fn copy_secondary(world: &mut World, source: CopySource) {
    if source.amount <= 0.0 {
        return;
    }
    for policy in COPIES {
        let eligible = source
            .spell
            .map_or(policy.white_hits, |spell| policy.spells.contains(&spell));
        if !eligible {
            continue;
        }
        let Some((amount, cap)) = read_copy_parameters(world, &source, policy) else {
            continue;
        };
        deliver_secondary(world, &source, policy, amount, cap);
    }
}

fn read_secondary_targets(
    world: &mut World,
    source: &CopySource,
    policy: &SecondaryCopy,
    cap: usize,
) -> Vec<Entity> {
    let center = targets::position(world, source.caster).expect("secondary-copy caster position");
    targets::area_units(
        world,
        source.caster,
        center,
        policy.radius,
        targets::Relation::Enemy,
        0,
    )
    .into_iter()
    .filter(|target| *target != source.primary)
    .take(cap)
    .collect()
}

fn deliver_secondary(
    world: &mut World,
    source: &CopySource,
    policy: &SecondaryCopy,
    amount: f32,
    cap: usize,
) {
    let recipients = read_secondary_targets(world, source, policy, cap);
    let spell = world
        .resource::<SpellStore>()
        .get(policy.payload)
        .expect("native secondary-copy payload")
        .clone();
    let ctx = EffectCtx {
        caster: source.caster,
        spell: &spell,
        depth: source.depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    for target in recipients {
        let versus = effects::creature_type_damage_multiplier(world, source.caster, target, &spell);
        effects::deal_copied_damage(world, &ctx, target, amount * versus);
    }
}
