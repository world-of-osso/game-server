//! Bounded consumers of local Mainline12.1.0.69933 fields and named children.
use super::{batch8, script_effect_value};
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::effects::{self, EffectCtx};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

const FROST_STRIKE: u32 = 49143;
const OBLITERATE: u32 = 49020;
pub(super) const RISING_SUN_DAMAGE: u32 = 185099;
const RISING_SUN_RANK: u32 = 128595;
const MORTAL_WOUNDS: u32 = 115804;
const HIGH_TIDE: u32 = 288675;
const COMBUSTION: u32 = 190319;
const PYROMANIAC: u32 = 1254512;
const BEACON: u32 = 53563;
const BEACON_PERCENT: u32 = 53651;
const BEACON_HEAL: u32 = 53652;

///49143 variable609: $owb==0 uses325464; dual wield uses222026+66196.
pub(in crate::spell_cast) fn effect_enabled(
    world: &World,
    caster: Entity,
    spell: u32,
    index: u8,
) -> bool {
    let weapon_branch = (spell == FROST_STRIKE && (1..=3).contains(&index))
        || (spell == OBLITERATE && matches!(index, 1..=3 | 6..=8));
    if !weapon_branch {
        return true;
    }
    let dual = world
        .get::<crate::stats::CharacterStats>(caster)
        .is_some_and(|stats| stats.off_hand.is_some());
    //49020 variable608 names both physical and frost counterparts per weapon branch.
    if matches!(index, 3 | 8) { !dual } else { dual }
}

///128595 explicitly grants Mortal Wounds to the Rising Sun Kick primary.
pub(super) fn wound_rising_sun(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if crate::auras::has_aura(world, caster, RISING_SUN_RANK)
        && batch8::first_damage_target(world, caster, RISING_SUN_DAMAGE, target)
    {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: MORTAL_WOUNDS,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

///288675s2's flat30 removes1064s2's30-percent loss. Native charged spell
///modifier consumption already spends one charge per completed Chain Heal.
pub(super) fn chain_falloff(world: &World, caster: Entity, falloff: f32) -> f32 {
    let reduction = if crate::auras::has_aura(world, caster, HIGH_TIDE) {
        script_effect_value(world, caster, HIGH_TIDE, 1)
    } else {
        0.0
    };
    1.0 - (falloff - reduction).max(0.0) / 100.0
}

///190319 names1254512s1: snapshot crit chance, not the unrelated aura189
///mastery-rating field. Both direct and periodic fire modifiers use this value.
pub(crate) fn snapshot_combustion(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    index: u8,
    amount: f32,
) -> f32 {
    if spell.id != COMBUSTION
        || !matches!(index, 2 | 3)
        || !crate::auras::has_aura(world, caster, PYROMANIAC)
    {
        return amount;
    }
    let crit = world
        .get::<crate::stats::CharacterStats>(caster)
        .map_or(0.0, |stats| stats.crit_pct);
    amount + crit * script_effect_value(world, caster, PYROMANIAC, 0) / 100.0
}

///53563 wraps a single ally; replacing this owner's mark preserves other owners.
pub(crate) fn replace_beacon(world: &mut World, unit: Entity, aura: &crate::auras::AuraInstance) {
    let previous: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(target, _)| *target != unit)
        .flat_map(|(target, auras)| {
            auras
                .iter()
                .filter(|mark| mark.spell_id == BEACON && mark.caster == aura.caster)
                .map(move |mark| (target, mark.instance_id))
        })
        .collect();
    for (target, instance) in previous {
        crate::auras::remove_instance(
            world,
            target,
            instance,
            crate::auras::RemoveReason::Cancelled,
        );
    }
}

fn same_group(world: &World, a: Entity, b: Entity) -> bool {
    let (Some(a), Some(b), Some(groups)) = (
        world.get::<crate::persistence::ClientId>(a),
        world.get::<crate::persistence::ClientId>(b),
        world.get_resource::<crate::group::GroupRegistry>(),
    ) else {
        return false;
    };
    groups.is_in_same_group(a.0, b.0)
}

///53563: only effective direct healing on another party/raid member transfers.
///53651s1 supplies10%;53652 carries the heal identity, not another power/crit roll.
pub(in crate::spell_cast) fn heal_beacon(
    world: &mut World,
    ctx: &EffectCtx,
    healed: Entity,
    effective: f32,
) {
    if ctx.spell.id == BEACON_HEAL || effective <= 0.0 || !same_group(world, ctx.caster, healed) {
        return;
    }
    let recipients: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(unit, auras)| {
            *unit != healed
                && auras
                    .iter()
                    .any(|aura| aura.spell_id == BEACON && aura.caster == ctx.caster)
        })
        .map(|(unit, _)| unit)
        .collect();
    if recipients.is_empty() {
        return;
    }
    let child = world
        .resource::<crate::spell_info::SpellStore>()
        .get(BEACON_HEAL)
        .expect("Beacon requires53652")
        .clone();
    let amount = effective * script_effect_value(world, ctx.caster, BEACON_PERCENT, 0) / 100.0;
    let transfer = effects::EffectCtx {
        caster: ctx.caster,
        spell: &child,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for unit in recipients {
        if same_group(world, ctx.caster, unit)
            && world.get::<crate::maps::InMap>(unit) == world.get::<crate::maps::InMap>(ctx.caster)
        {
            effects::apply_fixed_heal(world, &transfer, unit, amount);
        }
    }
}
