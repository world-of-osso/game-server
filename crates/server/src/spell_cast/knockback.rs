//! TC a352b1fa SpellEffects.cpp:4037-4075, effects98/144.
use super::{effects::EffectCtx, *};
use crate::spell_info::SpellEffect;
use shared::components::{Npc, Player, UnitSummonedBy};

const SPEED_RATIO: f32 = 0.1;
const MINIMUM_SPEED: f32 = 0.01;
const WORLD_BOSS_TYPE_FLAG: u32 = 0x4;
const DUNGEON_BOSS_EXTRA_FLAG: u32 = 0x1000_0000;
const KNOCKBACK_IMMUNITY_EXTRA_FLAG: u32 = 0x4000_0000;

pub(super) fn knock_back(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    if !target_allows_knockback(world, ctx.caster, target) {
        return;
    }
    let speed_xy = effect.misc_values[0] as f32 * SPEED_RATIO;
    let speed_z = effects::modded_base_points(world, ctx.caster, ctx.spell, effect) * SPEED_RATIO;
    if speed_xy.abs() < MINIMUM_SPEED && speed_z.abs() < MINIMUM_SPEED {
        return;
    }
    let origin = if effect.effect == 144 {
        ctx.dest.map(|destination| destination.position)
    } else {
        targets::position(world, ctx.caster)
    };
    let Some(origin) = origin else {
        error!(
            "Spell{} knockback has no source position/destination",
            ctx.spell.id
        );
        return;
    };
    match crate::knockback::launch(world, target, origin, speed_xy, speed_z) {
        Ok(()) => emit_knockback_proc(world, ctx.caster, target),
        Err(error) => error!("Spell{} knockback on {target:?}: {error}", ctx.spell.id),
    }
}

fn target_allows_knockback(world: &World, caster: Entity, target: Entity) -> bool {
    let rooted = world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras.has_type(crate::auras::handlers::MOD_ROOT)
                || auras.has_type(crate::auras::handlers::MOD_ROOT_2)
        });
    if rooted {
        return false;
    }
    let Some(npc) = world.get::<Npc>(target) else {
        return true;
    };
    let Some(template) = world
        .get_resource::<crate::world_data::WorldData>()
        .and_then(|data| data.templates.get(&npc.template_id))
    else {
        error!(
            "Knockback target entry{} has no sourced creature flags",
            npc.template_id
        );
        return false;
    };
    let controller = world.get::<UnitSummonedBy>(caster).map_or(caster, |owner| {
        Entity::try_from_bits(owner.0).expect("server summon controller")
    });
    let player_cast = world.get::<Player>(controller).is_some();
    let world_boss = template.type_flags & WORLD_BOSS_TYPE_FLAG != 0
        && world.get::<crate::hunter_pet::HunterPet>(target).is_none();
    let dungeon_boss = template.flags_extra & DUNGEON_BOSS_EXTRA_FLAG != 0;
    let immune = template.flags_extra & KNOCKBACK_IMMUNITY_EXTRA_FLAG != 0;
    !immune && !(player_cast && (world_boss || dungeon_boss))
}

fn emit_knockback_proc(world: &mut World, caster: Entity, target: Entity) {
    crate::auras::procs::push_event(
        world,
        crate::auras::procs::AuraEvent {
            actor: caster,
            target: Some(target),
            actor_mask: [0, 0],
            target_mask: [0, crate::auras::procs::PROC2_KNOCKBACK],
            spell_id: None,
            can_proc: true,
            periodic: false,
            damage: 0.0,
            reflected: false,
        },
    );
}
