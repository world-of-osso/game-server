//! TC a352b1fa SpellAuraEffects.cpp1194–1249: label Points modifiers
//! recalculate existing self-owned passive/permanent effects, not aura lifetimes.
use bevy::prelude::*;

use super::{ActiveAuras, AuraEffectState, AuraInstance, SpellModOp, handlers, spell_mods};
use crate::spell_info::{SpellInfo, SpellStore};

fn is_label_points(effect: &AuraEffectState) -> bool {
    const POINTS_OPERATIONS: &[SpellModOp] = &[
        SpellModOp::Points,
        SpellModOp::PointsIndex0,
        SpellModOp::PointsIndex1,
        SpellModOp::PointsIndex2,
        SpellModOp::PointsIndex3,
        SpellModOp::PointsIndex4,
    ];
    let label_modifier = matches!(
        effect.aura_type,
        handlers::ADD_FLAT_MODIFIER_BY_SPELL_LABEL | handlers::ADD_PCT_MODIFIER_BY_SPELL_LABEL
    );
    let points_operation = POINTS_OPERATIONS
        .iter()
        .any(|op| *op as i32 == effect.misc_values[0]);
    label_modifier && points_operation
}

fn uses_charges(world: &World, source: &AuraInstance) -> bool {
    // spend_charge removes the last charge after its stored count reaches zero.
    let native_charges = world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(source.spell_id))
        .and_then(|spell| spell.aura_options.as_ref())
        .is_some_and(|options| options.proc_charges > 0);
    source.charges > 0 || native_charges
}

struct PointModifier<'a> {
    aura: &'a AuraInstance,
    effect: &'a AuraEffectState,
    triggered_by: (u32, u8),
}

fn can_recalculate(unit: Entity, aura: &AuraInstance, source: &AuraInstance) -> bool {
    // SpellInfo.cpp2025–2035: Overflowing Maelstrom is the temporary-aura exception.
    const OVERFLOWING_MAELSTROM: u32 = 384669;
    let eligible_lifetime = aura.passive || aura.duration_ms == 0;
    aura.caster == unit && (eligible_lifetime || source.spell_id == OVERFLOWING_MAELSTROM)
}

fn affects_effect(
    modifier: &PointModifier<'_>,
    aura: &AuraInstance,
    state: &AuraEffectState,
    spell: &SpellInfo,
) -> bool {
    if (aura.instance_id, state.index) == modifier.triggered_by {
        return false;
    }
    let operation = modifier.effect.misc_values[0];
    let selected_index =
        SpellModOp::points_index(state.index).is_some_and(|op| operation == op as i32);
    let selected = operation == SpellModOp::Points as i32 || selected_index;
    selected && spell_mods::affects(modifier.aura, modifier.effect, spell)
}

struct AmountUpdate {
    instance: u32,
    index: u8,
    amount: f32,
}

fn compute_update(
    world: &World,
    modifier: &PointModifier<'_>,
    aura: &AuraInstance,
    state: &AuraEffectState,
    spell: &SpellInfo,
) -> Option<AmountUpdate> {
    // SpellInfo::IsAffectedBySpellMods, SpellInfo.cpp1988–1991.
    const IGNORE_CASTER_MODIFIERS: u32 = 0x2000_0000;
    if super::attributes(spell, 3) & IGNORE_CASTER_MODIFIERS != 0
        || !affects_effect(modifier, aura, state, spell)
    {
        return None;
    }
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == state.index)?;
    let amount =
        super::build_effect(world, aura.caster, spell, effect, super::aura_now(world)).amount;
    (amount != state.amount).then_some(AmountUpdate {
        instance: aura.instance_id,
        index: state.index,
        amount,
    })
}

fn collect_updates(world: &World, unit: Entity, modifier: &PointModifier<'_>) -> Vec<AmountUpdate> {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    auras
        .iter()
        .filter(|aura| can_recalculate(unit, aura, modifier.aura))
        .flat_map(|aura| {
            let spell = super::lookup_spell(world, aura.spell_id);
            aura.effects.iter().filter_map(move |state| {
                compute_update(world, modifier, aura, state, spell.as_ref()?)
            })
        })
        .collect()
}

fn apply_updates(
    world: &mut World,
    unit: Entity,
    updates: Vec<AmountUpdate>,
) -> Vec<(AuraInstance, AuraEffectState)> {
    if updates.is_empty() {
        return Vec::new();
    }
    let mut auras = world
        .get_mut::<ActiveAuras>(unit)
        .expect("collected point-modified auras");
    for update in &updates {
        let aura = auras
            .get_mut(update.instance)
            .expect("collected point-modified instance");
        let effect = aura
            .effects
            .iter_mut()
            .find(|effect| effect.index == update.index)
            .expect("collected point-modified effect");
        effect.amount = update.amount;
    }
    auras.dirty = true;
    collect_changed_modifiers(&auras, &updates)
}

fn collect_changed_modifiers(
    auras: &ActiveAuras,
    updates: &[AmountUpdate],
) -> Vec<(AuraInstance, AuraEffectState)> {
    updates
        .iter()
        .filter_map(|update| {
            let aura = auras
                .get(update.instance)
                .expect("updated point-modified instance");
            let effect = aura
                .effects
                .iter()
                .find(|effect| effect.index == update.index)
                .expect("updated point-modified effect");
            is_label_points(effect).then(|| (aura.clone(), effect.clone()))
        })
        .collect()
}

fn recalculate_modifier(world: &mut World, unit: Entity, modifier: &PointModifier<'_>) {
    if uses_charges(world, modifier.aura) {
        return;
    }
    let updates = collect_updates(world, unit, modifier);
    let changed_modifiers = apply_updates(world, unit, updates);
    // RecalculateAmount can itself change a spell modifier; retain TC's original
    // triggering effect exclusion while propagating its new amount to dependants.
    for (aura, effect) in changed_modifiers {
        let changed = PointModifier {
            aura: &aura,
            effect: &effect,
            triggered_by: modifier.triggered_by,
        };
        recalculate_modifier(world, unit, &changed);
    }
}

/// Called after the source is inserted/changed/removed; CalcValue sees live modifiers.
/// Caller's ordinary auras_changed pass reconciles stats and replication afterwards.
pub(super) fn recalculate(world: &mut World, unit: Entity, source: &AuraInstance) {
    for effect in source
        .effects
        .iter()
        .filter(|effect| is_label_points(effect))
    {
        let modifier = PointModifier {
            aura: source,
            effect,
            triggered_by: (source.instance_id, effect.index),
        };
        recalculate_modifier(world, unit, &modifier);
    }
}
