//! Native Eye Beam/Gaze channel children and the learned Eye Beam critical rank.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::ContextHitHook};
use shared::casting::{CastState, CastType};

const EYE_BEAM: u32 = 198013;
const GAZE: u32 = 452497;
const EYE_BEAM_DAMAGE: u32 = 198030;
const EYE_BEAM_RANK: u32 = 320415;
const EMPOWERED_BEAM: u32 = 1271144;
const EMPOWERED_DAMAGE: u32 = 1287949;

#[derive(Component)]
struct ChannelTick {
    parent: u32,
    child: u32,
}

// SimC76f8118 sc_demon_hunter.cpp4020–4028 chooses one tick action at channel start.
pub(super) fn snapshot_channel(world: &mut World, caster: Entity, spell: u32) {
    if !super::is_havoc(world, caster) || !matches!(spell, EYE_BEAM | GAZE) {
        return;
    }
    let child = if super::has_talent(world, caster, 1270898)
        && combat::auras::has_aura(world, caster, EMPOWERED_BEAM)
    {
        EMPOWERED_DAMAGE
    } else {
        EYE_BEAM_DAMAGE
    };
    world.entity_mut(caster).insert(ChannelTick {
        parent: spell,
        child,
    });
}

fn channel_child(world: &World, caster: Entity, parent: u32) -> Option<u32> {
    world
        .get::<ChannelTick>(caster)
        .filter(|snapshot| snapshot.parent == parent)
        .map(|snapshot| snapshot.child)
}

pub(crate) fn clear_channel(world: &mut World, caster: Entity, parent: u32) {
    if channel_child(world, caster, parent).is_some() {
        world.entity_mut(caster).remove::<ChannelTick>();
    }
}

// SimC76f8118 sc_demon_hunter.cpp3285–3327: a completed channel
// snapshots one more actual tick, scaled by the selected native talent.
pub(crate) fn finish_final_breath(world: &mut World, caster: Entity, parent: u32) {
    if !super::has_talent(world, caster, 1266500) {
        return;
    }
    let Some(child) = channel_child(world, caster, parent) else {
        error!(?caster, parent, "Final Breath has no channel tick snapshot");
        return;
    };
    let multiplier = spell_cast::scripts::script_effect_value(world, caster, 1266500, 0) / 100.0;
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        child,
        None,
        1,
        spell_cast::TriggeredSpellOverrides {
            damage_multiplier: Some(multiplier),
            ..Default::default()
        },
    ) {
        error!(?caster, child, "Final Breath native child refused");
    }
}

pub(super) fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (matches!(spell, EYE_BEAM | GAZE) && index == 0).then_some(cast_tick)
}

///198013's local variable316 names198030;320415 explicitly guarantees crit.
///No rank means ordinary generic crit, not a forced noncritical hit.
fn cast_tick(world: &mut World, ctx: &EffectCtx<'_>, _unit: Entity) {
    let caster = ctx.caster;
    let parent = ctx.spell.id;
    let channel = world
        .get::<CastState>(caster)
        .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == parent);
    if !channel {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3925–4050: both channels use native
    // cone children, not a hand-built single-target plan or the self cast unit.
    let Some(child) = channel_child(world, caster, parent) else {
        error!(?caster, parent, "Eye Beam has no channel tick snapshot");
        return;
    };
    spell_cast::cast_triggered(world, caster, child, None, 1);
}

fn isolated(world: &World, caster: Entity, spell: u32) -> bool {
    super::has_talent(world, caster, 388113)
        && spell_cast::resolving_spell_targets(world, caster, spell).is_some_and(|targets| {
            targets
                .iter()
                .filter(|&&target| {
                    spell_cast::targets::relation_matches(
                        world,
                        caster,
                        target,
                        spell_cast::targets::Relation::Enemy,
                    )
                })
                .count()
                == 1
        })
}

// SimC76f8118 sc_demon_hunter.cpp3939–3946,4651–4670,4705–4717,4799–4810.
pub(super) fn isolated_damage(
    world: &World,
    ctx: &EffectCtx<'_>,
    _: Entity,
    periodic: bool,
) -> f32 {
    if !periodic
        && matches!(ctx.spell.id, 198030 | 1287949)
        && isolated(world, ctx.caster, ctx.spell.id)
    {
        1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, 388113, 1) / 100.0
    } else {
        1.0
    }
}

pub(super) fn crit_chance(world: &World, caster: Entity, _: Entity, spell: u32) -> Option<f32> {
    if matches!(spell, 258921 | 258922) && isolated(world, caster, spell) {
        return Some(100.0);
    }
    (spell == EYE_BEAM_DAMAGE && combat::auras::has_aura(world, caster, EYE_BEAM_RANK))
        .then_some(100.0)
}
