//! Bounded consumers from local Mainline12.1.0.69933 rows/descriptions.
use super::{batch8, script_effect_value};
use crate::auras::{AuraApplication, AuraSource};
use crate::spell_cast::{
    self, PlannedEffect,
    effects::{CastEffectHandler, EffectCtx},
    targets,
};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;
use shared::components::{Player, PowerType};

pub(super) const ORB_DAMAGE: u32 = 84721;
pub(super) const BLIZZARD_DAMAGE: u32 = 190357;
const CHILLED: u32 = 205708;
const BLIZZARD_SLOW: u32 = 12486;
pub(super) const SURVIVAL_COMMAND: u32 = 259489;
const COMMAND_DAMAGE: u32 = 259277;
pub(super) const WILDFIRE_BOMB: u32 = 259495;
const BOMB_DAMAGE: u32 = 265157;
const BOMB_BLEED: u32 = 269747;
pub(crate) const DEATHMARK: u32 = 360194;
pub(crate) const EYE_BEAM: u32 = 198013;
pub(crate) const EYE_BEAM_DAMAGE: u32 = 198030;
const EYE_BEAM_RANK: u32 = 320415;
const METAMORPHOSIS: u32 = 191427;
const LANDING_HIT: u32 = 200166;
const LANDING_DAZE: u32 = 247121;
//191427's description explicitly says enemies within8yd; native Dummy radius
//is5yd and200166 has no radius, so neither supplies the landing geometry.
const LANDING_RADIUS: f32 = 8.0;
pub(super) const CONSUME: u32 = 473662;
const CONSUME_FURY: u32 = 1261710;

fn apply_owned(world: &mut World, caster: Entity, target: Entity, id: u32, mask: u32) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

///84714 and190356 name different slow spells/durations. Refresh only on hits.
pub(super) fn slow_orb(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if landed(world, caster, ORB_DAMAGE, target) {
        apply_owned(world, caster, target, CHILLED, 1);
    }
}
pub(super) fn slow_blizzard(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if landed(world, caster, BLIZZARD_DAMAGE, target) {
        apply_owned(world, caster, target, BLIZZARD_SLOW, 1);
    }
}
fn landed(world: &World, caster: Entity, spell: u32, target: Entity) -> bool {
    world
        .resource::<spell_cast::ResolvingHits>()
        .0
        .last()
        .is_some_and(|frame| {
            frame.caster == caster
                && frame.spell_id == spell
                && frame.damage.iter().any(|(unit, _)| *unit == target)
        })
}

///259489 names259277; the actual owned pet is the damage caster, not the owner.
pub(super) fn command_pet(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    cast_pet_command(world, caster, target, COMMAND_DAMAGE, depth);
}

/// 70391 spell34026 names83381; SimC76f8118 executes that attack on the owned pet.
///Animal Companion, charging and the other talent consumers are separate paths.
pub(super) fn command_beast(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    const BEAST_COMMAND_DAMAGE: u32 = 83381;
    cast_pet_command(world, caster, target, BEAST_COMMAND_DAMAGE, depth);
}

fn cast_pet_command(world: &mut World, caster: Entity, target: Entity, spell: u32, depth: u8) {
    if let Some(pet) = crate::hunter_pet::summoned_pet(world, caster).filter(|&pet| {
        world
            .get::<crate::hunter_pet::HunterPet>(pet)
            .is_some_and(|pet| pet.owner == caster)
    }) {
        spell_cast::cast_triggered(world, pet, spell, Some(target), depth + 1);
    }
}

///259495s3 adds its authored percentage to the named primary hit. The179 cone actor/cap remains open;
///this bridge does not create synthetic cone geometry or unbound actors.
pub(super) fn bomb_primary(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let child = world
        .resource::<SpellStore>()
        .get(BOMB_DAMAGE)
        .expect("Wildfire requires265157")
        .clone();
    let multiplier = 1.0 + script_effect_value(world, caster, WILDFIRE_BOMB, 2) / 100.0;
    let mut plan = spell_cast::plan_targets(world, caster, &child, Some(target), false)
        .expect("native primary bomb target");
    for effect in &mut plan {
        effect.chain_multipliers = vec![(target, multiplier)];
    }
    spell_cast::missiles::launch(world, caster, &child, plan, depth + 1);
    spell_cast::cast_triggered(world, caster, BOMB_BLEED, Some(target), depth + 1);
}

///360194s3 Energy over d, distributed on its native bleed clock. No separate
///timer survives the owned mark. Duration16s/period2s authors eight10-unit gains.
pub(crate) fn deathmark_energy(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let duration = spell_cast::duration_ms(world, caster, spell);
    let period = spell
        .effects
        .iter()
        .find(|e| e.index == 0)
        .expect("Deathmark bleed")
        .aura_period_ms;
    let total = script_effect_value(world, caster, DEATHMARK, 2);
    spell_cast::powers::change_power(
        world,
        caster,
        PowerType::Energy,
        total * period as f32 / duration as f32,
    );
}

///198013's local variable316 names198030;320415 explicitly guarantees crit.
///No rank means ordinary generic crit, not a forced noncritical hit.
pub(crate) fn cast_eye_beam_tick(world: &mut World, caster: Entity) {
    let Some(target) = world
        .get::<shared::casting::CastState>(caster)
        .filter(|cast| {
            cast.cast_type == shared::casting::CastType::Channel && cast.spell_id == EYE_BEAM
        })
        .and_then(|cast| Entity::try_from_bits(cast.target))
    else {
        return;
    };
    let from = targets::position(world, caster).expect("Eye Beam caster position");
    let Some(to) = targets::position(world, target) else {
        return;
    };
    let angle = targets::orientation(world, caster);
    let forward = Vec3::new(angle.cos(), 0.0, -angle.sin());
    let child = world
        .resource::<SpellStore>()
        .get(EYE_BEAM_DAMAGE)
        .expect("Eye Beam requires198030")
        .clone();
    if !targets::is_alive(world, target)
        || !targets::relation_matches(world, caster, target, targets::Relation::Enemy)
        || forward.dot(to - from) < 0.0
        || from.distance(to) > targets::effect_radius(&child.effects[0])
    {
        return;
    }
    // Named damage to the channel's primary only. Secondary129 cone geometry
    // and the root's reduced-damage threshold remain uncertified.
    let mut plan = vec![PlannedEffect {
        index: 0,
        units: vec![target],
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    spell_cast::hit::remove_defended_targets(world, caster, &child, &mut plan);
    spell_cast::missiles::launch(world, caster, &child, plan, 1);
}

pub(super) fn eye_beam_crit(world: &World, caster: Entity) -> Option<f32> {
    crate::auras::has_aura(world, caster, EYE_BEAM_RANK).then_some(100.0)
}

pub(in crate::spell_cast) fn cast_effect(spell: u32, index: u8) -> Option<CastEffectHandler> {
    match (spell, index) {
        (METAMORPHOSIS, 0) => Some(land_metamorphosis),
        _ => None,
    }
}
///Named damage/stun at the validated destination remains independent of effect254.
///Its source progress-curve displacement is unsupported; transform triggers still run.
fn land_metamorphosis(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &crate::spell_info::SpellEffect,
) {
    let center = ctx.dest.expect("Metamorphosis destination");
    let units = targets::area_units(
        world,
        ctx.caster,
        center.position,
        LANDING_RADIUS,
        targets::Relation::Enemy,
        0,
    );
    let child = world
        .resource::<SpellStore>()
        .get(LANDING_HIT)
        .expect("Metamorphosis requires200166")
        .clone();
    let damage_index = child
        .effects
        .iter()
        .position(|e| e.index == 1)
        .expect("landing damage");
    let stun_index = child
        .effects
        .iter()
        .position(|e| e.index == 0)
        .expect("landing stun");
    let npc_units = units
        .iter()
        .copied()
        .filter(|&u| world.get::<Player>(u).is_none())
        .collect();
    for &unit in &units {
        if world.get::<Player>(unit).is_some() {
            apply_owned(world, ctx.caster, unit, LANDING_DAZE, 1);
        }
    }
    let plan = vec![
        PlannedEffect {
            index: damage_index,
            units,
            dest: Some(center),
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
        PlannedEffect {
            index: stun_index,
            units: npc_units,
            dest: Some(center),
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
    ];
    spell_cast::missiles::launch(world, ctx.caster, &child, plan, ctx.depth + 1);
}

///473662 explicitly names1261710s1; no guessed optional fragment action.
pub(super) fn consume_fury(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if batch8::first_damage_target(world, caster, CONSUME, target) {
        spell_cast::cast_triggered(world, caster, CONSUME_FURY, Some(caster), depth + 1);
    }
}
