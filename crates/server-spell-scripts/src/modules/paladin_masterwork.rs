//! SimC76f8118 sc_paladin.cpp2442–2573, 2740–2811 and sc_paladin_protection.cpp
//! 448–462, 636–658: Masterwork forges Lesser Weapon charges.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraApplication, AuraInstance, AuraSource, procs::AuraEvent, scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::targets::{self, Relation, relation_matches};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
    SpellScriptHook::LateAuraRegistration(register),
    // Hammer casts spend Masterwork: Weapon; its native cast proc never fires.
    SpellScriptHook::OwnsPolicyProc(|spell| {
        matches!(spell, MASTERWORK_WEAPON | MASTERWORK_BULWARK)
    }),
];

const MASTERWORK: u32 = 1271387;
const MASTERWORK_WEAPON: u32 = 1271436;
const MASTERWORK_BULWARK: u32 = 1271383;
const LESSER_BULWARK: u32 = 1239002;
const LESSER_WEAPON: u32 = 1239091;
const LESSER_WEAPON_DAMAGE: u32 = 1239282;
const LESSER_WEAPON_HEAL: u32 = 1239276;
// sc_paladin.cpp2778–2786: each forged Lesser Weapon adds five charges.
const LESSER_WEAPON_CHARGES: u8 = 5;

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(LESSER_WEAPON).or_default().on_damage_proc = Some(strike_with_lesser_weapon);
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    match spell.id {
        // Sacred Weapon hardcasts; Divine Inspiration applies the aura without a cast.
        432472 => grant_masterwork_weapon(world, caster),
        432459 => grant_masterwork_bulwark(world, caster),
        // Blessed Hammer and Hammer of the Righteous.
        204019 | 53595 => {
            forge_lesser_weapon(world, caster);
            forge_lesser_armament(world, caster, MASTERWORK_BULWARK, LESSER_BULWARK);
        }
        20473 | 35395 => forge_holy_lesser_armaments(world, caster),
        _ => {}
    }
}

fn grant_masterwork_bulwark(world: &mut World, caster: Entity) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == MASTERWORK)
    }) {
        return;
    }
    let stacks = combat::spell_cast::scripts::script_effect_value(world, caster, MASTERWORK, 0);
    add_stacks(world, caster, MASTERWORK_BULWARK, stacks as u8);
}

// INFERRED_WCL dtmp7Lg2abzyZVBG fight 3 pull 1: Holy Shock at 12.701s
// spends Bulwark Masterwork and shields the Frost DK at 12.717s. SimC76f8118
// sc_paladin.cpp2757–2798 prefers other damage dealers for Lesser Armaments.
fn forge_holy_lesser_armaments(world: &mut World, caster: Entity) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 65)
    {
        return;
    }
    for (counter, child) in [
        (MASTERWORK_BULWARK, LESSER_BULWARK),
        (MASTERWORK_WEAPON, LESSER_WEAPON),
    ] {
        forge_lesser_armament(world, caster, counter, child);
    }
}

fn forge_lesser_armament(world: &mut World, caster: Entity, counter: u32, child: u32) {
    let Some((instance, _)) = owned_stacks(world, caster, counter) else {
        return;
    };
    let Some(target) = find_lesser_armament_target(world, caster, child) else {
        return;
    };
    combat::auras::spend_stack(world, caster, instance);
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: child,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    if child == LESSER_WEAPON {
        combat::auras::set_owned_stacks(world, caster, target, child, LESSER_WEAPON_CHARGES);
    }
}

fn find_lesser_armament_target(world: &mut World, caster: Entity, child: u32) -> Option<Entity> {
    let origin = targets::position(world, caster)?;
    let radius = {
        let spell = world.resource::<SpellStore>().get(child)?;
        targets::radius(&spell.effects[0], 1)
    };
    let mut units = targets::area_units(world, caster, origin, radius, Relation::Ally, 0);
    units.retain(|unit| world.get::<ActiveSpec>(*unit).is_some());
    units.sort_by_key(|unit| {
        let role = world
            .get::<ActiveSpec>(*unit)
            .and_then(|spec| {
                shared::class_spec::SPECIALIZATIONS
                    .iter()
                    .find(|entry| entry.id == spec.0)
            })
            .map(|entry| entry.role);
        let role_order = match role {
            Some(shared::class_spec::Role::MeleeDps | shared::class_spec::Role::RangedDps) => 0,
            Some(shared::class_spec::Role::Healer) => 1,
            _ => 2,
        };
        let already_has = world
            .get::<combat::auras::ActiveAuras>(*unit)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == child && aura.caster == caster)
            });
        (*unit == caster, already_has, role_order, unit.to_bits())
    });
    units.first().copied()
}

/// sc_paladin.cpp2740–2747: hardcast and Blessing of the Forge Sacred Weapons.
pub(super) fn grant_masterwork_weapon(world: &mut World, caster: Entity) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == MASTERWORK)
    }) {
        return;
    }
    let stacks = combat::spell_cast::scripts::script_effect_value(world, caster, MASTERWORK, 0);
    add_stacks(world, caster, MASTERWORK_WEAPON, stacks as u8);
}

fn forge_lesser_weapon(world: &mut World, caster: Entity) {
    let Some((masterwork, _)) = owned_stacks(world, caster, MASTERWORK_WEAPON) else {
        return;
    };
    combat::auras::spend_stack(world, caster, masterwork);
    add_stacks(world, caster, LESSER_WEAPON, LESSER_WEAPON_CHARGES);
}

fn strike_with_lesser_weapon(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let other = if event.actor == unit {
        event.target
    } else {
        Some(event.actor)
    };
    let target = other.or(event.action_target).unwrap_or(unit);
    let spell = if relation_matches(world, unit, target, Relation::Enemy) {
        LESSER_WEAPON_DAMAGE
    } else {
        LESSER_WEAPON_HEAL
    };
    combat::spell_cast::cast_triggered(world, unit, spell, Some(target), 1);
    combat::auras::spend_stack(world, unit, aura.instance_id);
}

fn owned_stacks(world: &World, caster: Entity, spell: u32) -> Option<(u32, u8)> {
    world
        .get::<combat::auras::ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
        .map(|aura| (aura.instance_id, aura.stacks))
}

/// SimC buff trigger(n): refresh the lifetime and add n stacks up to the native cap.
fn add_stacks(world: &mut World, caster: Entity, spell: u32, stacks: u8) {
    let before = owned_stacks(world, caster, spell).map_or(0, |(_, stacks)| stacks);
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    combat::auras::set_self_cast_stacks(world, caster, spell, before.saturating_add(stacks));
}
