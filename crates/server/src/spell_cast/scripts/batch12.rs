//! Bounded consumers of local Mainline rows; archive guardian bindings remain authoritative.
use bevy::prelude::*;
use shared::components::PowerType;

use super::script_effect_value;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::{self, CostPool, PlannedEffect, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const FERAL_SPIRIT: u32 = 51533;
const SPIRIT_WOLVES: u32 = 228562;
pub(super) const GARGOYLE_STRIKE: u32 = 51963;
const SUMMON_GARGOYLE: u32 = 49206;
const GARGOYLE_TALENT: u32 = 1242147;
const GARGOYLE_POWER: u32 = 211947;
pub(super) const SHIELD_RIGHTEOUS: u32 = 53600;
const SHIELD_ARMOR: u32 = 132403;
pub(super) const BUTCHERY: u32 = 212436;
const BLOODY_CLAWS: u32 = 459868;
const BUTCHERY_BLEED: u32 = 459870;

///51533 names228562 once; EffectSummonType owns its native two-wolf count
///(TC SpellEffects.cpp1934–1936, SummonProperties1161).
///The source's movement/physical/maelstrom variants are separate contracts.
pub(super) fn summon_wolves(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, SPIRIT_WOLVES, Some(target), depth + 1);
}

///1242147's always-critical attack is limited to its actual49206 guardian.
pub(super) fn gargoyle_crit(world: &World, caster: Entity) -> Option<f32> {
    let summon = world.get::<spell_cast::summon::SummonedCreature>(caster)?;
    (summon.spell_id == SUMMON_GARGOYLE
        && crate::auras::has_aura(world, summon.owner, GARGOYLE_TALENT))
    .then_some(100.0)
}

///Successful owner costs, not a later pool difference or arbitrary resource drains.
pub(in crate::spell_cast) fn empower_gargoyles(
    world: &mut World,
    owner: Entity,
    costs: &[(CostPool, f32)],
) {
    let spent = spell_cast::resources::spent(costs, PowerType::RunicPower);
    if spent <= 0.0 || !crate::auras::has_aura(world, owner, GARGOYLE_TALENT) {
        return;
    }
    let actors = owned_gargoyles(world, owner);
    if actors.is_empty() {
        return;
    }
    let display = world
        .resource::<spell_cast::powers::PowerTypeTable>()
        .get(PowerType::RunicPower)
        .expect("native Runic Power row")
        .display_modifier as f32;
    let points = script_effect_value(world, owner, SUMMON_GARGOYLE, 3);
    let percent = script_effect_value(world, owner, GARGOYLE_POWER, 0);
    let increase = spent / display / points * percent;
    for actor in actors {
        apply_gargoyle_power(world, owner, actor, increase);
    }
}

fn owned_gargoyles(world: &mut World, owner: Entity) -> Vec<Entity> {
    world
        .query::<(Entity, &spell_cast::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(actor, summon)| {
            let owned = summon.owner == owner && summon.spell_id == SUMMON_GARGOYLE;
            let same_map =
                world.get::<crate::maps::InMap>(*actor) == world.get::<crate::maps::InMap>(owner);
            owned && same_map && targets::is_alive(world, *actor)
        })
        .map(|(actor, _)| actor)
        .collect()
}

fn apply_gargoyle_power(world: &mut World, owner: Entity, actor: Entity, increase: f32) {
    let current = world.get::<ActiveAuras>(actor).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == GARGOYLE_POWER && aura.caster == owner)
    });
    let previous = current
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    let mut spell = world
        .resource::<SpellStore>()
        .get(GARGOYLE_POWER)
        .expect("native Gargoyle damage aura")
        .clone();
    spell.effects[0].base_points = previous + increase;
    crate::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster: owner,
            target: actor,
            spell_id: GARGOYLE_POWER,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &spell,
    );
}

///53600's literal front cone: native6yd radius/120 degrees, not target-centric AoE.
///Ally relation makes an untargeted cast validate on its caster, not an invented enemy requirement.
pub(super) fn shield_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    _explicit: Option<Entity>,
) -> spell_cast::TargetPlan {
    vec![PlannedEffect {
        index: 0,
        units: shield_cone_units(world, caster, spell),
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn shield_cone_units(world: &mut World, caster: Entity, spell: &SpellInfo) -> Vec<Entity> {
    let origin = targets::position(world, caster).expect("shield caster position");
    let facing = targets::orientation(world, caster);
    let forward = Vec3::new(facing.cos(), 0.0, -facing.sin());
    let half_arc = spell
        .target_restrictions
        .as_ref()
        .expect("native shield cone")
        .cone_degrees
        .to_radians()
        / 2.0;
    let mut units = targets::area_units(
        world,
        caster,
        origin,
        targets::effect_radius(&spell.effects[0]),
        targets::Relation::Enemy,
        0,
    );
    units.retain(|&unit| {
        let offset = targets::position(world, unit).expect("cone unit position") - origin;
        within_shield_arc(offset, forward, half_arc)
    });
    units
}

fn within_shield_arc(offset: Vec3, forward: Vec3, half_arc: f32) -> bool {
    let along = offset.dot(forward);
    let across = (offset.x * forward.z - offset.z * forward.x).abs();
    across.atan2(along) <= half_arc
}

///53600 explicitly names132403s1/d. Apply once, including empty cones.
pub(super) fn shield_armor(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: SHIELD_ARMOR,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
}

///212436's tooltip gates its named459870 bleed on459868.
pub(super) fn butchery_bleed(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if crate::auras::has_aura(world, caster, BLOODY_CLAWS) {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: BUTCHERY_BLEED,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}
