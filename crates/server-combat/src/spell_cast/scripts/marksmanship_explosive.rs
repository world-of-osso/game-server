//! SimC76f8118 Explosive Shot primary-tick relationships for Marksmanship.
use crate::auras;
use crate::spell_cast::{PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::ActiveSpec;

const EXPLOSIVE: u32 = 212431;
const CLEAVE: u32 = 212680;
const MID2_FOUR_PIECE: u32 = 1296634;

pub(in crate::spell_cast) fn after_tick(
    world: &mut World,
    ctx: &EffectCtx,
    primary: Entity,
    damage: f32,
) {
    if ctx.spell.id != EXPLOSIVE
        || damage <= 0.0
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 254)
    {
        return;
    }
    // sc_hunter.cpp5893–5900: the real primary tick supplies the budget;
    // native root effect3 owns the excluded share, child212680 owns the area.
    let excluded = super::script_effect_value(world, ctx.caster, EXPLOSIVE, 2) / 100.0;
    super::super::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        CLEAVE,
        Some(primary),
        ctx.depth + 1,
        damage * (1.0 - excluded),
    );
    // sc_hunter.cpp5901–5906: only the actual equipped four-piece supplies
    // effect2 milliseconds, once per primary tick rather than each cleave hit.
    if auras::has_aura(world, ctx.caster, MID2_FOUR_PIECE) {
        let seconds = f64::from(super::script_effect_value(
            world,
            ctx.caster,
            MID2_FOUR_PIECE,
            1,
        )) / 1000.0;
        for spell in [19434, 257044] {
            super::reduce_cooldown(world, ctx.caster, spell, seconds);
        }
    }
}

/// sc_hunter.cpp5872–5878: only secondary recipients; root effect2 supplies
/// the reduced-AoE cap. Coefficients and geometry remain native.
pub(super) fn cleave_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Explosive cleave retains its producing tick's primary");
    let center = targets::position(world, primary).expect("actual Explosive primary position");
    let radius = targets::radius(&spell.effects[0], 1);
    let mut units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    units.retain(|&unit| unit != primary);
    let cap = super::script_effect_value(world, caster, EXPLOSIVE, 1);
    let multiplier = if cap > 0.0 && units.len() as f32 > cap {
        (cap / units.len().min(20) as f32).sqrt()
    } else {
        1.0
    };
    let chain_multipliers = units.iter().map(|&unit| (unit, multiplier)).collect();
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

pub(super) fn target_plan(spell: u32) -> Option<(targets::Relation, super::TargetPlanHook)> {
    (spell == CLEAVE).then_some((targets::Relation::Enemy, cleave_plan))
}
