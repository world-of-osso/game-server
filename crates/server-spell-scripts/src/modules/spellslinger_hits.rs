//! Spellslinger impact relationships; native70448 owns fractions, counts and geometry.
use super::combat;
use bevy::prelude::*;
use shared::components::ActiveSpec;

use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    targets,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_damage)];
const CONTROLLED_INSTINCTS: u32 = 444483;
const INFUSED_SPLINTERS: u32 = 1261080;

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, _critical: bool, amount: f32) {
    let Some(spec) = world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0) else {
        return;
    };
    let (source, payload, fraction_effect) = match spec {
        62 => (443763, 444720, 0),
        64 => (443722, 444487, 3),
        _ => return,
    };
    if ctx.spell.id != source {
        return;
    }
    if auras::has_aura(world, ctx.caster, CONTROLLED_INSTINCTS) {
        // SimC76f8118 sc_mage.cpp5340–5362/5400–5403: copy the actual
        // resolved source hit, without a second source multiplier or crit roll.
        let fraction =
            script_effect_value(world, ctx.caster, CONTROLLED_INSTINCTS, fraction_effect) / 100.0;
        copy_secondaries(world, ctx, target, payload, amount * fraction);
    }
    if spec == 64 {
        roll_freezing(world, ctx.caster, target);
    }
}

fn roll_freezing(world: &mut World, caster: Entity, target: Entity) {
    if !auras::has_aura(world, caster, INFUSED_SPLINTERS) {
        return;
    }
    // sc_mage.cpp5380–5381/1891–1892: native infused effect2 chance and
    // effect4 count apply Freezing to the actual landed Splinter recipient.
    let chance = script_effect_value(world, caster, INFUSED_SPLINTERS, 1);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let count = script_effect_value(world, caster, INFUSED_SPLINTERS, 3) as u32;
    for _ in 0..count {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: 1221389,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn copy_secondaries(
    world: &mut World,
    source: &EffectCtx,
    primary: Entity,
    payload: u32,
    amount: f32,
) {
    let spell = world
        .resource::<SpellStore>()
        .get(payload)
        .expect("native Controlled Instincts payload")
        .clone();
    let recipients = select_secondaries(world, source.caster, primary, &spell);
    let ctx = EffectCtx {
        caster: source.caster,
        spell: &spell,
        depth: source.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for target in recipients {
        effects::deal_copied_damage(world, &ctx, target, amount);
    }
}

fn select_secondaries(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    spell: &SpellInfo,
) -> Vec<Entity> {
    let center = targets::position(world, primary).expect("landed Splinter recipient position");
    let radius = targets::radius(&spell.effects[0], 1);
    // sc_mage.cpp5346–5354: the pinned default bugs=1 uses a hard cap,
    // not a soft reduced-AoE multiplier. Always exclude the source primary.
    let cap = script_effect_value(world, caster, CONTROLLED_INSTINCTS, 4) as usize;
    targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0)
        .into_iter()
        .filter(|target| *target != primary)
        .take(cap)
        .collect()
}
