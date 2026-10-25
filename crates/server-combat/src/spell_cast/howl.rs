//! SIMC_BM_HOWL_OF_THE_PACK_LEADER_CYCLE: Cry is earned by a real native summon.
//! SimC76f8118 hunter3783 supplies root-effect2 stacks; DB2 supplies amounts/cap/clock.
use bevy::prelude::*;

use super::effects::EffectCtx;
use super::summon::SummonedCreature;

const HOWL: u32 = 471876;
const WYVERN_SUMMON: u32 = 1222271;
const WYVERN_CRY: u32 = 471881;

pub(super) fn on_summoned(world: &mut World, ctx: &EffectCtx, actor: Entity) {
    if ctx.spell.id != WYVERN_SUMMON || !crate::auras::has_aura(world, ctx.caster, HOWL) {
        return;
    }
    let actual_summon = world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| summon.owner == ctx.caster && summon.spell_id == WYVERN_SUMMON);
    if !actual_summon {
        return;
    }
    let stacks = super::scripts::script_effect_value(world, ctx.caster, HOWL, 2) as u8;
    if stacks == 0 {
        return;
    }
    let cast = super::cast_triggered_with_overrides(
        world,
        ctx.caster,
        WYVERN_CRY,
        Some(ctx.caster),
        ctx.depth.saturating_add(1),
        super::TriggeredSpellOverrides {
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    );
    if !cast {
        warn!(owner = ?ctx.caster, ?actor, "actual Howl Wyvern could not apply native Cry");
    }
}
