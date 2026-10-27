//! Source-qualified Incinerate primary, Fire and Brimstone and Havoc impacts.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    PlannedEffect, TargetPlan,
    script_registry::{SpellScriptHook, TargetPlanHook},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const INCINERATE: u32 = 29722;
const FIRE_AND_BRIMSTONE: u32 = 196408;
const HAVOC: u32 = 80240;
const DAMAGE_EFFECT: u8 = 1;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetPlan(target_plan)];

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == INCINERATE).then_some((Relation::Enemy, plan_incinerate))
}

fn plan_incinerate(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let destruction = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 267);
    let primary = explicit.expect("native Incinerate requires its primary target");
    let fnb = destruction && has_talent(world, caster, FIRE_AND_BRIMSTONE);
    let fnb_fraction = if fnb {
        native_fraction(world, FIRE_AND_BRIMSTONE)
    } else {
        0.0
    };
    let havoc = if destruction {
        find_owned_havoc(world, caster, primary)
    } else {
        None
    };
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.effect != 0)
        .map(|(index, effect)| {
            let native_units =
                targets::select_targets(world, caster, explicit, spell, effect, 0, false)
                    .expect("native Incinerate effect target selectors");
            let (units, chain_multipliers) = if destruction && effect.index == DAMAGE_EFFECT {
                select_damage_targets(primary, native_units, fnb, fnb_fraction, havoc)
            } else {
                (native_units, Vec::new())
            };
            PlannedEffect {
                index,
                units,
                selected_units: Vec::new(),
                dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
                chain_multipliers,
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            }
        })
        .collect()
}

fn has_talent(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn native_fraction(world: &World, spell: u32) -> f32 {
    world
        .resource::<SpellStore>()
        .get(spell)
        .expect("selected native Incinerate talent")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Incinerate talent effect0")
        .base_points
        / 100.0
}

fn find_owned_havoc(world: &mut World, caster: Entity, primary: Entity) -> Option<(Entity, f32)> {
    let now = combat::auras::aura_now(world);
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .find_map(|(target, auras)| {
            let valid = target != primary
                && targets::is_alive(world, target)
                && targets::same_map(world, caster, target)
                && targets::relation_matches(world, caster, target, Relation::Enemy);
            if !valid {
                return None;
            }
            let mark = auras.iter().find(|aura| {
                aura.spell_id == HAVOC
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|expiry| expiry > now)
            })?;
            let fraction = mark
                .effects
                .iter()
                .find(|effect| effect.index == 0)
                .expect("native Havoc effect0")
                .amount
                / 100.0;
            Some((target, fraction))
        })
}

fn select_damage_targets(
    primary: Entity,
    candidates: Vec<Entity>,
    fnb: bool,
    fnb_fraction: f32,
    havoc: Option<(Entity, f32)>,
) -> (Vec<Entity>, Vec<(Entity, f32)>) {
    // SimC76f8118 actions3852–3900/3952–4002: primary once, FnB has
    // uncapped native area secondaries, and Havoc is separate even out of area.
    // The Havoc recipient receives both sourced percentages, never two hits.
    let mut units = vec![primary];
    if fnb {
        units.extend(
            candidates
                .into_iter()
                .filter(|&candidate| candidate != primary),
        );
    }
    if let Some((target, _)) = havoc
        && !units.contains(&target)
    {
        units.push(target);
    }
    let multipliers = units
        .iter()
        .map(|&target| {
            let fraction = if target == primary {
                1.0
            } else {
                fnb_fraction
                    + havoc
                        .filter(|(marked, _)| *marked == target)
                        .map_or(0.0, |(_, value)| value)
            };
            (target, fraction)
        })
        .collect();
    (units, multipliers)
}
