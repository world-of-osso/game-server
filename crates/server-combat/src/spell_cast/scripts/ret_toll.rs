//! SIMC_RET_DIVINE_TOLL_RELATIONSHIP: native geometry/amounts, sourced child edge.
//! SimC76f8118 sc_paladin.cpp1580–1590,1618–1631,1815–1935;
//! retribution.cpp1251–1253. TC/native metadata contain no dispatch edge.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_cast::targets::{self, Relation};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const ROOT: u32 = 375576;
const JUDGMENT: u32 = 406957;
const HAMMER: u32 = 1279408;
const HAMMER_MODE: u32 = 1277026;
const RETRIBUTION: u32 = 70;

fn native_cap(spell: &SpellInfo) -> Option<usize> {
    spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map(|effect| effect.base_points)
        .filter(|amount| amount.is_finite() && *amount > 0.0)
        .map(|amount| amount as usize)
}

/// The explicit target is already selected by effect0. Reserve that recipient,
/// and cap only the other native effect1 area selections; do not replace range,
/// visibility, combat admission or the planner's candidate ordering.
pub(super) fn limit_area(
    _world: &World,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let Some(cap) = native_cap(spell) else {
        warn!("Divine Toll native effect0 target cap unavailable");
        units.clear();
        return;
    };
    units.retain(|unit| Some(*unit) != explicit);
    units.truncate(cap.saturating_sub(usize::from(explicit.is_some())));
}

pub(super) fn cast_child(world: &mut World, ctx: &EffectCtx, target: Entity) {
    let is_ret = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == RETRIBUTION);
    if !is_ret || !targets::relation_matches(world, ctx.caster, target, Relation::Enemy) {
        return;
    }
    if native_cap(ctx.spell).is_none() {
        warn!("Divine Toll dispatch lacks its native target cap");
        return;
    }
    let id = if auras::has_aura(world, ctx.caster, HAMMER_MODE) {
        HAMMER
    } else {
        JUDGMENT
    };
    let Some(mut child) = world.resource::<SpellStore>().get(id).cloned() else {
        warn!(spell_id = id, "Divine Toll native child unavailable");
        return;
    };
    let percent = super::script_effect_value(world, ctx.caster, ROOT, 5);
    child.cast_damage_multiplier = Some(1.0 + percent / 100.0);
    // Same native triggered child seam as the other approved Ret relationships.
    // Funding and Resonance/Exaction/DivineHammer are separate qualifications.
    if !super::super::launch_triggered_spell(world, ctx.caster, &child, Some(target), ctx.depth + 1)
    {
        debug!(
            spell_id = id,
            ?target,
            "Divine Toll native child dispatch refused"
        );
    }
}
