//! Source-qualified effective resource values, separate from actual payment.
use super::combat;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, spell_mods};
use combat::random::Rng;
use combat::spell_cast;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceValueAfterCast(after_cast_hook),
    SpellScriptHook::TriggeredResourceProducer(apply_producers),
    SpellScriptHook::EffectiveResourceValue(effective_value),
    SpellScriptHook::ConsumeResourceValueSources(consume_sources),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];
use bevy::prelude::*;
use shared::components::PowerType;

struct MaximumValueRule {
    aura: u32,
    resource: PowerType,
    match_effect: u8,
    source: u32,
    producer_casts: &'static [u32],
    chance_effect: u8,
}

const RULES: &[MaximumValueRule] = &[
    // SimC76f8118 sc_rogue.cpp2021–2027 uses consume_cp_max for an action
    // affected by Fang Strike;1926 consumes that one use after execution.
    // The native aura effect supplies the affected family mask, not an action-ID list.
    MaximumValueRule {
        aura: 1301405,
        resource: PowerType::ComboPoints,
        match_effect: 0,
        // sc_rogue.cpp3597/5991/11265–11267: only Ambush/Sinister
        // execute the tier roll; native source effect0 owns its20% chance.
        source: 1296589,
        producer_casts: &[8676, 193315, 197834],
        chance_effect: 0,
    },
];

pub(super) fn owns_proc(source: u32) -> bool {
    RULES.iter().any(|rule| rule.source == source)
}

pub(super) fn after_cast_hook(parent: u32) -> Option<AfterCastHook> {
    RULES
        .iter()
        .any(|rule| rule.producer_casts.contains(&parent))
        .then_some(apply_producers as AfterCastHook)
}

fn apply_producers(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for rule in RULES
        .iter()
        .filter(|rule| rule.producer_casts.contains(&spell.id))
    {
        let Some((chance, payload)) = read_producer(world, caster, rule) else {
            continue;
        };
        if combat::random::thread_rng().gen_range(0.0..100.0) < chance {
            let depth = spell_cast::next_trigger_depth(world);
            spell_cast::cast_triggered(world, caster, payload, Some(caster), depth);
        }
    }
}

fn read_producer(world: &World, caster: Entity, rule: &MaximumValueRule) -> Option<(f32, u32)> {
    let now = spell_cast::now(world);
    let source = world.get::<ActiveAuras>(caster)?.iter().find(|aura| {
        aura.caster == caster
            && aura.spell_id == rule.source
            && aura.expires_at.is_none_or(|expiry| expiry > now)
    })?;
    let effect = source
        .effects
        .iter()
        .find(|effect| effect.index == rule.chance_effect)
        .expect("native resource-window source must retain its chance/trigger effect");
    assert_ne!(
        effect.trigger_spell, 0,
        "native resource-window source requires a payload"
    );
    Some((effect.amount, effect.trigger_spell))
}

fn matches(world: &World, caster: Entity, spell: &SpellInfo, rule: &MaximumValueRule) -> bool {
    if !spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == rule.resource as i32)
    {
        return false;
    }
    let now = spell_cast::now(world);
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == rule.aura
                && aura.caster == caster
                && aura.expires_at.is_none_or(|expiry| expiry > now)
                && affects(aura, spell, rule.match_effect)
        })
    })
}

fn affects(aura: &AuraInstance, spell: &SpellInfo, index: u8) -> bool {
    aura.effects
        .iter()
        .any(|effect| effect.index == index && spell_mods::class_mask_affects(aura, effect, spell))
}

fn effective_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    resource: PowerType,
    paid: f32,
) -> f32 {
    let maximum = RULES
        .iter()
        .any(|rule| rule.resource == resource && matches(world, caster, spell, rule));
    if maximum {
        return spell_cast::powers::power_value(world, caster, resource)
            .expect("effective maximum resource requires the native pool")
            .1;
    }
    paid
}

fn consume_sources(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let sources: Vec<_> = RULES
        .iter()
        .filter(|rule| matches(world, caster, spell, rule))
        .map(|rule| rule.aura)
        .collect();
    for source in sources {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == source && aura.caster == caster
        });
    }
}
