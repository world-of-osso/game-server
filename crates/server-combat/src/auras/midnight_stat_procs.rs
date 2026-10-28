//! SimC76f8118 unique_gear_midnight.cpp: equipped stat effects whose native
//! rows do not carry the applied value (Arcanoweave Insight, Eversong Diamond).
use bevy::prelude::*;

use super::{
    ActiveAuras, AuraApplication, AuraInstance, AuraSource,
    procs::AuraEvent,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_info::SpellStore;

const LINING: u32 = 1283697;
const LINING_RPPM: u32 = 1229511;
const INSIGHT: u32 = 1229746;
const EVERSONG: u32 = 1258209;
/// unique_gear_midnight.cpp45–55: Peridot, Garnet, Lapis, Amethyst descriptions.
const GEM_COLORS: [u32; 4] = [14262, 14276, 14277, 14278];

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        LINING_RPPM,
        AuraScript {
            check_proc: Some(is_first_lining_callback),
            on_proc: Some(apply_insight),
            ..Default::default()
        },
    );
    scripts.0.insert(
        EVERSONG,
        AuraScript {
            on_apply: Some(apply_eversong_colors),
            ..Default::default()
        },
    );
}

fn count_unique_gem_colors(world: &World, unit: Entity) -> Option<usize> {
    let equipped = world.get::<crate::stats::EquippedItemModifiers>(unit)?;
    let native = world.get_resource::<crate::stats::NativeItemInstances>()?;
    let descriptions: std::collections::BTreeSet<u32> = equipped
        .items
        .iter()
        .flat_map(|(_, item)| item.gems.iter())
        .filter_map(|gem| native.enchants.gem_descriptions.get(gem).copied())
        .collect();
    Some(
        GEM_COLORS
            .iter()
            .filter(|color| descriptions.contains(color))
            .count(),
    )
}

/// unique_gear_midnight.cpp469–477: effects2/3 (native1/2: crit damage and
/// crit healing) become effect1 base value times the unique equipped colors.
fn apply_eversong_colors(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(colors) = count_unique_gem_colors(world, unit) else {
        error!(
            ?unit,
            "Eversong Diamond holder lacks native equipped gem data"
        );
        return;
    };
    let Some(per_color) = world
        .resource::<SpellStore>()
        .get(EVERSONG)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| effect.base_points)
    else {
        error!(?unit, "native Eversong Diamond1258209 lacks effect0");
        return;
    };
    let Some(mut auras) = world.get_mut::<ActiveAuras>(unit) else {
        return;
    };
    let Some(live) = auras.get_mut(aura.instance_id) else {
        return;
    };
    for effect in live
        .effects
        .iter_mut()
        .filter(|effect| matches!(effect.index, 1 | 2))
    {
        effect.amount = per_color * colors as f32;
    }
}

/// unique_gear_midnight.cpp687–689: equipped copies share one callback.
fn is_first_lining_callback(world: &World, aura: &AuraInstance, _event: &AuraEvent) -> bool {
    world.get::<ActiveAuras>(aura.caster).is_some_and(|auras| {
        auras
            .iter()
            .filter(|other| other.spell_id == LINING_RPPM)
            .map(|other| other.instance_id)
            .min()
            == Some(aura.instance_id)
    })
}

/// Item-scaled native effect value; effect1 is a non-aura dummy, so it is
/// read from the catalog at the equipped item level, not the live aura.
fn read_driver_value(
    world: &World,
    unit: Entity,
    item_level: Option<u16>,
    item_inventory_type: Option<u8>,
    index: u8,
) -> Option<f32> {
    let mut spell = world.resource::<SpellStore>().get(LINING)?.clone();
    spell.cast_item_level = item_level;
    spell.cast_item_inventory_type = item_inventory_type;
    let effect = spell.effects.iter().find(|effect| effect.index == index)?;
    Some(crate::spell_cast::effects::modded_base_points(
        world, unit, &spell, effect,
    ))
}

/// unique_gear_midnight.cpp682–685: each equipped driver adds its effect0
/// value times its effect1 player share to the one buff.
fn read_insight_amount(world: &World, unit: Entity) -> Option<f32> {
    let sources: Vec<_> = world
        .get::<ActiveAuras>(unit)?
        .iter()
        .filter(|aura| aura.spell_id == LINING && matches!(aura.source, AuraSource::Equipment(_)))
        .map(|aura| (aura.cast_item_level, aura.cast_item_inventory_type))
        .collect();
    sources
        .into_iter()
        .try_fold(0.0, |sum, (level, inventory_type)| {
            let value = read_driver_value(world, unit, level, inventory_type, 0)?;
            let share = read_driver_value(world, unit, level, inventory_type, 1)?;
            Some(sum + value * share / 100.0)
        })
}

fn apply_insight(world: &mut World, unit: Entity, _rppm: &AuraInstance, _other: Option<Entity>) {
    let Some(amount) = read_insight_amount(world, unit).filter(|amount| *amount > 0.0) else {
        error!(
            ?unit,
            "Arcanoweave Lining driver lacks native effect0/effect1 value"
        );
        return;
    };
    let Some(mut buff) = world.resource::<SpellStore>().get(INSIGHT).cloned() else {
        error!(?unit, "native Arcanoweave Insight1229746 absent");
        return;
    };
    let Some(effect) = buff.effects.iter_mut().find(|effect| effect.index == 0) else {
        error!(?unit, "native Arcanoweave Insight1229746 lacks effect0");
        return;
    };
    effect.base_points = amount;
    super::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: INSIGHT,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &buff,
    );
}
