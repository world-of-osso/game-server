//! Partial Golden Serpent port from TC a352b1fa boss_golden_serpent.cpp.
//! TC script schedule (partial; Retail 12.1 timeline inputs unavailable).
//! Also the Serpentine Seal -> Shadow of Zul temple event and Shadow-Borne temple
//! trash from kings_rest.cpp.
//! Intro fly anim tier/hover state, GO state visuals, near-point collision-angle
//! search and native visuals remain gaps.
mod gold_follow;
mod golden_serpent;
mod gust_slash;
mod molten_gold;
mod serpentine_seal;
mod shadow_of_zul;
mod temple_trash;
use super::{CreatureAi, Ctx, DamageTaken};
use bevy::prelude::*;
pub(crate) use gold_follow::{
    load_model_reaches as load_gold_model_reaches, update_follows as update_gold_follows,
};
#[cfg(test)]
pub(crate) use gust_slash::GustSlashTrigger;
pub(crate) use gust_slash::{
    CREATE_PROPERTIES as GUST_SLASH_CREATE_PROPERTIES, create as create_gust_slash,
    load_shape as load_gust_slash_shape, update as update_gust_slashes,
};
pub(crate) use molten_gold::{
    handles_effect as handles_molten_gold_effect, load_areas as load_molten_gold_areas,
    update_areas as update_molten_gold_areas,
};
pub(crate) use serpentine_seal::{
    SCRIPT as SERPENTINE_SEAL_SCRIPT, gossip_hello as serpentine_seal_gossip_hello,
    update as update_serpentine_seals,
};
use shared::components::UnitFlags;

pub(crate) const ANIMATED_GOLD_SCRIPT: &str = "npc_animated_gold";
pub(crate) const GOLDEN_SERPENT_SCRIPT: &str = "boss_the_golden_serpent";
pub(crate) const GOLDEN_SERPENT_ENCOUNTER: u32 = 2139;
const INSTANCE: &str = "instance_kings_rest";
const MOLTEN_GOLD: u32 = 265915;
const BREAK_TARGETING: u32 = 140562;
const SPIT_GOLD: u32 = 265773;
const LUCRES_CALL: u32 = 265923;
const LUSTER: u32 = 265991;
const INVISIBLE_DISPLAY: u32 = 11686;
const TASK_FOLLOW: u32 = 1;
const FOLLOW_DELAY_MS: u64 = 3_000;
const ABSORPTION_DESPAWN_MS: u32 = 4_000;
const ANIMATED_GOLD: u32 = 135406;

pub(crate) fn is_creature_script(name: &str) -> bool {
    matches!(
        name,
        ANIMATED_GOLD_SCRIPT | GOLDEN_SERPENT_SCRIPT | shadow_of_zul::SCRIPT
    ) || temple_trash::is_script(name)
}

pub(crate) fn creature_ai(world: &World, actor: Entity, name: &str) -> Option<Box<dyn CreatureAi>> {
    if !is_creature_script(name) {
        return None;
    }
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return None;
    }
    if name == GOLDEN_SERPENT_SCRIPT {
        return golden_serpent::creature_ai(world, actor);
    }
    if name == shadow_of_zul::SCRIPT {
        return Some(Box::new(shadow_of_zul::ShadowOfZul::default()));
    }
    if temple_trash::is_script(name) {
        return temple_trash::creature_ai(name);
    }
    Some(Box::new(AnimatedGold::default()))
}

#[derive(Default)]
struct AnimatedGold {
    molten_cast: bool,
    tasks: super::EventMap,
}

impl AnimatedGold {
    fn start_follow(&mut self, ctx: &mut Ctx) {
        let molten = ctx
            .world
            .get::<crate::auras::ActiveAuras>(ctx.me)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == MOLTEN_GOLD));
        if molten {
            return;
        }
        let Some(primary) = gold_follow::primary(ctx.world, ctx.me) else {
            return;
        };
        gold_follow::start(ctx.world, ctx.me, primary);
        self.molten_cast = false;
    }
}

impl CreatureAi for AnimatedGold {
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        ctx.do_cast_self(BREAK_TARGETING);
        ctx.set_react_state(super::ReactState::Passive);
        ctx.world
            .entity_mut(ctx.me)
            .insert(super::ScriptedVictimUpdatePaused);
    }
    fn damage_taken(&mut self, ctx: &mut Ctx, _attacker: Option<Entity>, damage: DamageTaken) {
        if damage.damage >= damage.health_before && !self.molten_cast {
            ctx.do_cast_self(MOLTEN_GOLD);
            gold_follow::clear(ctx.world, ctx.me);
            super::movement::clear_point_movement(ctx.world, ctx.me);
            self.molten_cast = true;
        }
    }
    fn spell_hit(&mut self, _ctx: &mut Ctx, _caster: Entity, spell_id: u32) {
        if spell_id == LUCRES_CALL {
            self.tasks.schedule(TASK_FOLLOW, FOLLOW_DELAY_MS);
        }
    }
    fn movement_inform(&mut self, ctx: &mut Ctx, kind: super::MotionType, _id: u32) {
        if kind != super::MotionType::Follow {
            return;
        }
        let Some(primary) = gold_follow::primary(ctx.world, ctx.me) else {
            return;
        };
        if ctx.world.get::<super::CreatureAiState>(primary).is_none() {
            return;
        }
        super::smart_ai::lifetimes::schedule_despawn(
            ctx.world,
            ctx.me,
            ABSORPTION_DESPAWN_MS,
            0,
            false,
        );
        gold_follow::clear(ctx.world, ctx.me);
        super::movement::clear_point_movement(ctx.world, ctx.me);
        ctx.do_cast_at(None, LUSTER, true);
        super::dispatch(
            ctx.world,
            primary,
            super::Hook::DoAction(golden_serpent::ACTION_ANNOUNCE_ABSORB),
        );
        let mut flags = *ctx.world.get::<UnitFlags>(ctx.me).expect("Gold unit flags");
        flags.0 |= UnitFlags::NOT_SELECTABLE;
        ctx.world.entity_mut(ctx.me).insert((
            shared::components::ModelDisplay {
                display_id: INVISIBLE_DISPLAY,
            },
            flags,
        ));
    }
    // TC companion UpdateAI only advances its task scheduler, never UpdateVictim.
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.tasks.update(u64::from(diff_ms));
        while let Some(TASK_FOLLOW) = self.tasks.execute_event() {
            self.start_follow(ctx);
        }
    }
}

/// `SPAWN_GROUP_PRE_FIRST_BOSS`: the TempleEvent trash Shadow of Zul raises.
pub(crate) const SPAWN_GROUP_PRE_FIRST_BOSS: u32 = 1255;
/// `SPAWN_GROUP_SERPENT_BOSS`: manual group spawned by the instance script.
pub(crate) const SPAWN_GROUP_SERPENT_BOSS: u32 = 1256;
const TEMPLE_EVENT: &str = "TempleEvent";

/// `instance_kings_rest::OnUnitDeath`: the copy's last living TempleEvent creature
/// dying spawns the Serpent's group.
pub(crate) fn spawn_serpent_after_temple_event(
    died: Query<
        (
            &crate::maps::InMap,
            &crate::zones::CreatureStringIds,
            Option<&crate::zones::CreatureDifficulty>,
            Has<lightyear::prelude::Replicate>,
        ),
        Added<crate::combat::DeathState>,
    >,
    living: Query<
        (&crate::maps::InMap, &crate::zones::CreatureStringIds),
        Without<crate::combat::DeathState>,
    >,
    mut commands: Commands,
) {
    for (&map, string_id, difficulty, replicate) in &died {
        let remaining = living
            .iter()
            .any(|(other, ids)| *other == map && ids.has(TEMPLE_EVENT));
        if !string_id.has(TEMPLE_EVENT) || remaining {
            continue;
        }
        let difficulty = difficulty.map_or(0, |difficulty| difficulty.0);
        commands.queue(move |world: &mut World| {
            spawn_serpent_group(world, map, difficulty, replicate);
        });
    }
}

fn spawn_serpent_group(
    world: &mut World,
    map: crate::maps::InMap,
    difficulty: u32,
    replicate: bool,
) {
    let maps = world.resource::<crate::maps::MapStore>();
    if maps.instance_scripts.get(&map.map_id).map(String::as_str) != Some(INSTANCE) {
        return;
    }
    // A killed boss keeps its respawn time, so SpawnGroupSpawn skips it.
    let done = super::instance_script::get_boss_state(world, map, 0)
        == Some(super::instance_script::EncounterState::Done);
    let present = world
        .query::<(&crate::maps::InMap, &shared::components::Npc)>()
        .iter(world)
        .any(|(other, npc)| *other == map && npc.template_id == golden_serpent::ENTRY);
    if done || present {
        return;
    }
    crate::zones::spawn_group_spawn(world, map, difficulty, replicate, SPAWN_GROUP_SERPENT_BOSS);
}

pub(crate) fn register_aura_scripts(mut scripts: ResMut<crate::auras::scripts::AuraScripts>) {
    scripts.0.insert(
        SPIT_GOLD,
        crate::auras::scripts::AuraScript {
            on_remove: Some(summon_gold_after_spit),
            ..Default::default()
        },
    );
    scripts.0.insert(
        temple_trash::SPELL_BOUND_BY_SHADOW,
        crate::auras::scripts::AuraScript {
            on_remove: Some(temple_trash::kill_when_shadow_broken),
            ..Default::default()
        },
    );
    scripts.0.insert(
        MOLTEN_GOLD,
        crate::auras::scripts::AuraScript {
            on_apply: Some(apply_molten_immunity),
            on_remove: Some(remove_molten_immunity),
            ..Default::default()
        },
    );
}

/// TC Spit Gold effect0/periodic-damage AfterEffectRemove: only death/expiry.
fn summon_gold_after_spit(
    world: &mut World,
    target: Entity,
    aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    if !matches!(
        reason,
        crate::auras::RemoveReason::Expired | crate::auras::RemoveReason::Death
    ) {
        return;
    }
    let periodic = aura.effects.iter().any(|effect| {
        effect.index == 0 && effect.aura_type == crate::auras::handlers::PERIODIC_DAMAGE
    });
    if !periodic || world.get_entity(aura.caster).is_err() {
        return;
    }
    let position = world
        .get::<shared::components::Position>(target)
        .expect("Spit Gold target position");
    let orientation = world
        .get::<shared::components::Rotation>(target)
        .map_or(0.0, |rotation| rotation.y - std::f32::consts::FRAC_PI_2);
    let location = [position.x, -position.z, position.y, orientation];
    crate::zones::summon_creature(world, aura.caster, ANIMATED_GOLD, location);
}

fn has_transform(aura: &crate::auras::AuraInstance) -> bool {
    aura.effects
        .iter()
        .any(|effect| effect.index == 1 && effect.aura_type == 56)
}

fn apply_molten_immunity(world: &mut World, actor: Entity, aura: &crate::auras::AuraInstance) {
    molten_gold::apply(world, actor, aura);
    if has_transform(aura) {
        set_molten_flags(world, actor, true);
    }
}
fn remove_molten_immunity(
    world: &mut World,
    actor: Entity,
    aura: &crate::auras::AuraInstance,
    _reason: crate::auras::RemoveReason,
) {
    molten_gold::remove(world, actor, aura);
    if has_transform(aura) {
        set_molten_flags(world, actor, false);
    }
}
fn set_molten_flags(world: &mut World, actor: Entity, apply: bool) {
    let mask = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;
    let mut flags = world
        .get::<UnitFlags>(actor)
        .copied()
        .unwrap_or(UnitFlags(0));
    if apply {
        flags.0 |= mask;
    } else {
        flags.0 &= !mask;
    }
    world.entity_mut(actor).insert(flags);
}
