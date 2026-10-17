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

/// TC a352b1fa SpellEffects.cpp:4151. Pull does not run EffectKnockBack immunities.
pub(super) fn pull_towards(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) {
    if let Err(error) = launch_pull(world, ctx, effect, target) {
        error!("Spell{} pull on {target:?}: {error}", ctx.spell.id);
    }
}

fn launch_pull(
    world: &mut World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
    target: Entity,
) -> Result<(), String> {
    let start = targets::position(world, target).ok_or("pull target has no position")?;
    let destination = probe_pull_destination(world, ctx.caster, start)?;
    // Blizzard intentionally uses 3D distance here, not projectile horizontal distance.
    let distance = start.distance(destination);
    if distance < 0.001 {
        return Ok(());
    }
    let speed_xy = if effect.misc_values[0] != 0 {
        effect.misc_values[0] as f32 * SPEED_RATIO
    } else {
        30.0 // TC EffectPullTowards default horizontal speed.
    };
    let height = destination.y - start.y;
    let speed_z = (2.0 * speed_xy * speed_xy * height
        + shared::skyriding::GRAVITY * distance * distance)
        / (2.0 * speed_xy * distance);
    if !speed_z.is_finite() {
        return Err("non-finite vertical pull speed".into());
    }
    let offset = destination - start;
    let direction = Vec3::new(offset.x, 0.0, offset.z).normalize_or_zero();
    crate::knockback::launch_directed(world, target, direction * speed_xy + Vec3::Y * speed_z)
}

/// Existing collision/ground services implement the supported direct-ray subset of
/// Object.cpp:2833. Detour raycasts and missing map obstacle bakes remain unproven.
fn probe_pull_destination(world: &mut World, caster: Entity, target: Vec3) -> Result<Vec3, String> {
    let start = targets::position(world, caster).ok_or("pull caster has no position")?;
    let map = *world
        .get::<crate::maps::InMap>(caster)
        .ok_or("pull caster has no map")?;
    let reach = crate::scripted_ai::model_reach::combat_reach(world, caster)
        .ok_or("pull caster has no sourced combat reach")?;
    let offset = target - start;
    let direction = Vec3::new(offset.x, 0.0, offset.z).normalize_or_zero();
    let destination = start + direction * reach;
    let los = world
        .get_resource::<crate::line_of_sight::LineOfSight>()
        .ok_or("pull collision service unavailable")?;
    let mut destination = super::teleport::clamp_to_wall(los, map, start, destination);
    let mut ground = world
        .get_resource_mut::<crate::ground::GroundMap>()
        .ok_or("pull ground service unavailable")?;
    let crate::ground::GroundProbe::Supported(height) = ground.creature_ground_in(map, destination)
    else {
        return Err("pull destination has no supported ground".into());
    };
    destination.y = height;
    Ok(destination)
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
