//! Creature scripts selected by `creature_template.ScriptName`, ported from
//! TrinityCore's `CreatureAI` / `ScriptedAI` / `BossAI` (game/AI/) with the
//! instance scripts they report to (`InstanceScript`, game/Instances/).
//!
//! A creature whose ScriptName is registered (`the_stockade::creature_ai`) gets a
//! `CreatureAiState` holding its `CreatureAi` (TrinityCore `RegisterCreatureAI`,
//! created by `init_creature_ai` and `Reset` like `InitializeAI`). The hooks are
//! called where the generic systems reach the TrinityCore call sites: engagement
//! in `creature_aggro::add_threat` (`JustEngagedWith`), evade in
//! `creature_aggro::enter_evade_mode` (`EnterEvadeMode` → `Reset`), death in
//! `combat::insert_npc_death` (`JustDied`), damage in the melee and spell damage
//! paths (`DamageTaken`), summons (`JustSummoned`, `SummonedCreatureDespawn`) and
//! point movement (`MovementInform`) and script paths (`WaypointPathEnded`). `UpdateAI` runs every tick of a living
//! creature. A hook raised while the creature's own AI runs (a script killing
//! itself) is queued and runs right after.
//! See `docs/specs/boss-scripts.md`.

use crate::random as rand;
mod algethar_academy;
pub(crate) mod azure_vault;
mod azureblade;
mod blood_furnace;
pub mod boss_ai;
pub mod bossmod;
mod broggok;
pub(crate) mod caster_chase;
pub mod creature_text;
mod doragosa;
pub mod event_map;
pub(crate) mod hellfire_ramparts;
pub mod instance_script;
pub(crate) mod jump_movement;
pub(crate) mod kargath;
pub mod kings_rest;
mod kokia;
pub(crate) mod maraudon;
mod melidrussa;
pub(crate) mod model_reach;
pub mod movement;
pub mod murder_row;
pub(crate) mod nethekurse;
pub(crate) mod nethekurse_intro;
pub(crate) mod ragefire_chasm;
pub(crate) mod ruby_life_pools;
pub(crate) mod shattered_halls;
pub mod smart_ai;
pub mod the_stockade;
mod umbrelskul;
pub(crate) mod wailing_caverns;
pub mod waypoint_path;

use std::collections::VecDeque;

use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Player, ThreatTable};

use crate::combat::DeathState;
use crate::creature_ai::CreatureState;
use crate::grid_activity::GridDormant;
use crate::maps::InMap;
use crate::networking::CombatTarget;

pub use event_map::EventMap;

/// Manual spawn groups ported scripts spawn; copy fill withholds their members.
pub(crate) const SCRIPT_SPAWN_GROUPS: &[u32] = &[
    kings_rest::SPAWN_GROUP_PRE_FIRST_BOSS,
    kings_rest::SPAWN_GROUP_SERPENT_BOSS,
    blood_furnace::SPAWN_GROUP_CHANNELER_1,
    blood_furnace::SPAWN_GROUP_CHANNELER_2,
    blood_furnace::SPAWN_GROUP_CHANNELER_3,
    blood_furnace::SPAWN_GROUP_CHANNELER_4,
    blood_furnace::SPAWN_GROUP_CHANNELER_5,
];

/// `gameobject_template.ScriptName`s with a ported `GameObjectAI`.
pub(crate) fn is_game_object_script(script_name: &str) -> bool {
    matches!(
        script_name,
        kings_rest::SERPENTINE_SEAL_SCRIPT | nethekurse_intro::DOOR_SCRIPT
    )
}

/// `GameObjectAI::OnGossipHello` of the object's ported script.
pub(crate) fn game_object_gossip_hello(world: &mut World, player: Entity, object: Entity) {
    let Some(script) = world.get::<crate::game_objects::GameObjectScript>(object) else {
        return;
    };
    if script.0 == kings_rest::SERPENTINE_SEAL_SCRIPT {
        kings_rest::serpentine_seal_gossip_hello(world, player, object);
    } else if script.0 == nethekurse_intro::DOOR_SCRIPT {
        nethekurse_intro::door_gossip_hello(world, object);
    }
}

pub(crate) mod fall_movement;

pub struct ScriptedAiPlugin;

impl Plugin for ScriptedAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::auras::scripts::AuraScripts>()
            .init_resource::<instance_script::InstanceScripts>()
            .add_systems(Startup, creature_text::load_creature_texts_startup)
            .add_systems(Startup, smart_ai::startup_load_smart_scripts)
            .add_systems(Startup, bossmod::startup_load_catalog)
            .add_systems(Startup, murder_row::register_aura_scripts)
            .add_systems(Startup, maraudon::register_aura_scripts)
            .add_systems(Startup, kings_rest::register_aura_scripts)
            .add_systems(Startup, ruby_life_pools::register_aura_scripts)
            .add_systems(Startup, azure_vault::register_aura_scripts)
            .add_systems(Startup, blood_furnace::register_aura_scripts)
            .add_systems(Startup, shattered_halls::register_aura_scripts)
            .add_systems(FixedPreUpdate, trigger_just_appeared)
            .add_systems(FixedPostUpdate, hellfire_ramparts::update_instance_resets)
            .add_systems(FixedPostUpdate, hellfire_ramparts::liquid_fire::update)
            .add_systems(FixedPreUpdate, jump_movement::release_finished_jumps)
            .add_systems(FixedPreUpdate, fall_movement::release_finished_falls)
            .add_systems(
                FixedUpdate,
                (
                    bossmod::advance_schedules,
                    init_creature_ai,
                    ragefire_chasm::intro_triggers::detect_entries,
                    nethekurse_intro::detect_entries,
                    update_creature_ai,
                )
                    .chain()
                    // TC Creature::Update runs Unit::Update before AIUpdateTick.
                    // A new AI cast starts aging on the following server tick.
                    .after(crate::spell_cast::advance_casts)
                    .after(crate::creature_aggro::update_victim_system)
                    .before(crate::creature_ai::chase_system)
                    .before(crate::combat::auto_attack_system),
            )
            .add_systems(
                FixedUpdate,
                (
                    smart_ai::lifetimes::tick_lifetimes_system.before(update_creature_ai),
                    movement::point_movement_system.before(crate::creature_ai::chase_system),
                    waypoint_path::path_movement_system.before(crate::creature_ai::chase_system),
                    jump_movement::advance_jumps
                        .after(movement::point_movement_system)
                        .after(waypoint_path::path_movement_system)
                        .before(update_creature_ai)
                        .before(crate::creature_ai::chase_system),
                    fall_movement::advance_falls
                        .after(movement::point_movement_system)
                        .after(waypoint_path::path_movement_system)
                        .before(update_creature_ai)
                        .before(crate::creature_ai::chase_system),
                    caster_chase::advance
                        .after(update_creature_ai)
                        .before(crate::creature_ai::chase_system),
                    kings_rest::update_gold_follows
                        .after(update_creature_ai)
                        .before(crate::creature_ai::chase_system),
                    kings_rest::spawn_serpent_after_temple_event,
                    kings_rest::update_serpentine_seals.before(update_creature_ai),
                    kings_rest::update_gust_slashes.after(update_creature_ai),
                    kings_rest::update_molten_gold_areas
                        .after(update_creature_ai)
                        .after(crate::combat::auto_attack_system),
                ),
            );
    }
}

/// `creature_template.ScriptName` of a spawned creature with a registered AI; empty
/// for a boss an instance script's `creatureData` names without a script of its own
/// (Grim Batol's).
#[derive(Component, Debug, Clone)]
pub struct CreatureScriptName(pub String);

/// A scripted phase does not call TC UpdateVictim: pause active-victim projection
/// and evade until combat updates resume. ThreatManager's reference still updates.
#[derive(Component)]
pub(crate) struct ScriptedVictimUpdatePaused;

/// `ReactStates`: a passive creature neither attacks on sight nor projects a new attack victim.
/// Its threat-manager current reference still updates independently
/// (`CreatureAI::UpdateVictim` stops its attack); an aggressive one does both.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReactState {
    Passive,
    #[default]
    Aggressive,
}

/// `MovementGeneratorType` of a finished movement (`MovementInform`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionType {
    /// `POINT_MOTION_TYPE`.
    Point,
    /// `FOLLOW_MOTION_TYPE`.
    Follow,
    /// `EFFECT_MOTION_TYPE`, including source MoveJump.
    Effect,
}

/// A `CreatureAI` hook with its arguments.
#[derive(Debug, Clone, Copy)]
pub enum Hook {
    InitializeAi,
    Reset,
    JustAppeared,
    ChannelFinished {
        spell_id: u32,
    },
    SpellCast {
        spell_id: u32,
    },
    JustEngagedWith(Entity),
    KilledUnit(Entity),
    JustDied(Option<Entity>),
    /// `DamageTaken(attacker, damage)` with the health the damage was taken from.
    DamageTaken {
        attacker: Option<Entity>,
        health_before: f32,
        damage: f32,
    },
    EnterEvadeMode,
    /// `HomeMovementGenerator::DoFinalize` → `CreatureAI::JustReachedHome`.
    JustReachedHome,
    JustSummoned(Entity),
    IsSummonedBy(Entity),
    SummonedCreatureDespawn(Entity),
    MovementInform(MotionType, u32),
    /// `WaypointPathEnded(nodeId, pathId)`.
    WaypointPathEnded {
        node_id: u32,
        path_id: u32,
    },
    DoAction(i32),
    /// SmartScript::StoreCounter on a selected SmartAI creature.
    StoreCounter {
        id: u32,
        value: u32,
        reset: bool,
    },
    UpdateAi(u32),
    QuestAccepted {
        player: Entity,
        quest_id: u32,
    },
    QuestRewarded {
        player: Entity,
        quest_id: u32,
    },
    /// `SpellHit(caster, spellInfo)`.
    SpellHit {
        caster: Entity,
        spell_id: u32,
    },
}

/// TrinityCore `CreatureAI` virtuals the Stockade scripts override. Defaults do
/// what `CreatureAI`/`ScriptedAI` do when a script does not override them.
pub trait CreatureAi: Send + Sync + 'static {
    fn initialize_ai(&mut self, ctx: &mut Ctx) {
        dispatch(ctx.world, ctx.me, Hook::Reset);
    }
    fn reset(&mut self, _ctx: &mut Ctx) {}
    fn just_appeared(&mut self, _ctx: &mut Ctx) {}
    fn channel_finished(&mut self, _ctx: &mut Ctx, _spell_id: u32) {}
    fn on_spell_cast(&mut self, _ctx: &mut Ctx, _spell_id: u32) {}
    fn just_engaged_with(&mut self, _ctx: &mut Ctx, _who: Entity) {}
    fn killed_unit(&mut self, _ctx: &mut Ctx, _victim: Entity) {}
    fn just_died(&mut self, _ctx: &mut Ctx, _killer: Option<Entity>) {}
    fn damage_taken(&mut self, _ctx: &mut Ctx, _attacker: Option<Entity>, _damage: DamageTaken) {}
    /// `CreatureAI::EnterEvadeMode`: the generic evade already ran
    /// (`creature_aggro::enter_evade_mode`), then `Reset`.
    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        self.reset(ctx);
    }
    fn just_reached_home(&mut self, _ctx: &mut Ctx) {}
    fn just_summoned(&mut self, _ctx: &mut Ctx, _summon: Entity) {}
    fn is_summoned_by(&mut self, _ctx: &mut Ctx, _summoner: Entity) {}
    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, _summon: Entity) {}
    fn movement_inform(&mut self, _ctx: &mut Ctx, _kind: MotionType, _id: u32) {}
    fn waypoint_path_ended(&mut self, _ctx: &mut Ctx, _node_id: u32, _path_id: u32) {}
    fn spell_hit(&mut self, _ctx: &mut Ctx, _caster: Entity, _spell_id: u32) {}
    fn do_action(&mut self, _ctx: &mut Ctx, _action: i32) {}
    fn gossip_hello(&mut self, _ctx: &mut Ctx, _player: Entity) -> bool {
        false
    }
    fn gossip_select(&mut self, _ctx: &mut Ctx, _player: Entity) -> bool {
        false
    }
    fn store_counter(&mut self, _ctx: &mut Ctx, _id: u32, _value: u32, _reset: bool) {}
    fn quest_accepted(&mut self, _ctx: &mut Ctx, _player: Entity, _quest_id: u32) {}
    fn quest_rewarded(&mut self, _ctx: &mut Ctx, _player: Entity, _quest_id: u32) {}
    /// `ScriptedAI::UpdateAI`: keep the victim current.
    fn update_ai(&mut self, ctx: &mut Ctx, _diff_ms: u32) {
        ctx.update_victim();
    }
}

/// The damage of one `DamageTaken` call and the health it was taken from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageTaken {
    pub health_before: f32,
    pub max_health: f32,
    pub damage: f32,
}

impl DamageTaken {
    /// `Unit::HealthBelowPctDamaged`: health minus the damage is below `pct` of max.
    pub fn health_below_pct_damaged(&self, pct: f32) -> bool {
        self.health_before - self.damage < self.max_health * pct / 100.0
    }

    /// `ScriptedAI::HealthAbovePct`, read while the damage is being dealt.
    pub fn health_above_pct(&self, pct: f32) -> bool {
        self.health_before > self.max_health * pct / 100.0
    }
}

/// A creature's AI and the hooks raised while it was running.
#[derive(Component)]
pub struct CreatureAiState {
    ai: Option<Box<dyn CreatureAi>>,
    pending: VecDeque<Hook>,
}

/// `ScriptMgr` registration by ScriptName, else the `AIName` SmartAI; `None` for names
/// without a ported script.
fn create_ai(world: &World, creature: Entity, script_name: &str) -> Option<Box<dyn CreatureAi>> {
    if script_name == SMART_AI || script_name.is_empty() {
        return smart_ai::creature_ai(world, creature);
    }
    if azure_vault::is_creature_script(script_name) {
        return azure_vault::creature_ai(world, creature, script_name);
    }
    if algethar_academy::is_creature_script(script_name) {
        return algethar_academy::creature_ai(world, creature, script_name);
    }
    if murder_row::is_creature_script(script_name) {
        return murder_row::creature_ai(world, creature, script_name);
    }
    if kings_rest::is_creature_script(script_name) {
        return kings_rest::creature_ai(world, creature, script_name);
    }
    if ragefire_chasm::is_creature_script(script_name) {
        return ragefire_chasm::creature_ai(world, creature, script_name);
    }
    if wailing_caverns::is_creature_script(script_name) {
        return wailing_caverns::creature_ai(world, creature);
    }
    if maraudon::is_creature_script(script_name) {
        return maraudon::creature_ai(world, creature);
    }
    if hellfire_ramparts::is_creature_script(script_name) {
        return hellfire_ramparts::creature_ai(world, creature);
    }
    if ruby_life_pools::is_creature_script(script_name) {
        return ruby_life_pools::creature_ai(world, creature);
    }
    if blood_furnace::is_creature_script(script_name) {
        return blood_furnace::creature_ai(world, creature);
    }
    if shattered_halls::is_creature_script(script_name) {
        return shattered_halls::creature_ai(world, creature);
    }
    if nethekurse::is_creature_script(script_name) {
        return nethekurse::creature_ai(world, creature);
    }
    if kargath::is_creature_script(script_name) {
        return kargath::creature_ai(world, creature);
    }
    the_stockade::creature_ai(world, creature, script_name)
}

/// `CreatureScriptName` of a creature running `SmartAI` (its `AIName`, no ScriptName).
const SMART_AI: &str = "SmartAI";

/// A spawned creature with a ported ScriptName, a scriptless Grim Batol boss
/// (`entry`) or SmartAI rows for its entry gets its AI as soon as it is in the world
/// (`Creature::AIM_Initialize` at `AddToMap`; `FactorySelector::SelectAI` prefers the
/// ScriptName).
pub(crate) fn insert_script_name(entity: &mut EntityCommands, script_name: &str, entry: u32) {
    let script_name = if the_stockade::is_creature_script(script_name)
        || murder_row::is_creature_script(script_name)
        || kings_rest::is_creature_script(script_name)
        || ragefire_chasm::is_creature_script(script_name)
        || wailing_caverns::is_creature_script(script_name)
        || maraudon::is_creature_script(script_name)
        || azure_vault::is_creature_script(script_name)
        || algethar_academy::is_creature_script(script_name)
        || ruby_life_pools::is_creature_script(script_name)
        || hellfire_ramparts::is_creature_script(script_name)
        || blood_furnace::is_creature_script(script_name)
        || shattered_halls::is_creature_script(script_name)
        || nethekurse::is_creature_script(script_name)
        || kargath::is_creature_script(script_name)
    {
        Some(script_name.to_owned())
    } else {
        None
    };
    entity.queue(move |entity: EntityWorldMut| {
        let creature = entity.id();
        entity
            .into_world_mut()
            .commands()
            .queue(move |world: &mut World| {
                let name = script_name.or_else(|| {
                    (smart_ai::has_script(world, entry)
                        || instance_script::bound_boss(world, creature).is_some())
                    .then(|| SMART_AI.to_owned())
                });
                let Some(name) = name else {
                    return;
                };
                world.entity_mut(creature).insert(CreatureScriptName(name));
                initialize_ai(world, creature);
            });
    });
}

/// `Creature::AIM_Create` + `InitializeAI` → `Reset`, for new and respawned creatures.
fn init_creature_ai(world: &mut World) {
    let pending: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<CreatureScriptName>,
            Without<CreatureAiState>,
            Without<DeathState>,
        )>()
        .iter(world)
        .collect();
    for creature in pending {
        initialize_ai(world, creature);
    }
}

fn initialize_ai(world: &mut World, creature: Entity) {
    let Some(name) = world
        .get::<CreatureScriptName>(creature)
        .map(|name| name.0.clone())
    else {
        return;
    };
    if world.get::<CreatureAiState>(creature).is_some() {
        return;
    }
    let Some(ai) = create_ai(world, creature, &name) else {
        // GetInstanceAI: outside its instance script's map the script does not run.
        world.entity_mut(creature).remove::<CreatureScriptName>();
        return;
    };
    // A new SmartAI starts with `_canCombatMove = true`; ordinary Reset keeps it.
    world
        .entity_mut(creature)
        .remove::<smart_ai::CombatMovementDisabled>()
        .insert(CreatureAiState {
            ai: Some(ai),
            pending: VecDeque::new(),
        });
    // SmartAI initializes event timers in its constructor and dispatches RESET
    // after RESPAWN in JustAppeared. Other CreatureAI implementations reset here.
    if name != SMART_AI && !name.is_empty() {
        dispatch(world, creature, Hook::InitializeAi);
    }
    world.entity_mut(creature).insert(TriggerJustAppeared);
}

/// `Creature::m_triggerJustAppeared`: set at AddToMap and Respawn, consumed by the
/// creature's next `Update`, so a summoner's post-summon setup (e.g.
/// `SetScriptStringId`) is visible to `JustAppeared`.
#[derive(Component)]
struct TriggerJustAppeared;

/// `Creature::Respawn`: the creature starts over with a new AI.
pub(crate) fn reinitialize(world: &mut World, creature: Entity) {
    if let Ok(mut entity) = world.get_entity_mut(creature) {
        entity.remove::<CreatureAiState>();
    }
}

/// `Creature::Update` (ALIVE) → `AIUpdateTick` → `UpdateAI(diff)`.
fn update_creature_ai(world: &mut World) {
    let diff_ms = world.resource::<Time>().delta().as_millis() as u32;
    let creatures: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<CreatureAiState>,
            Without<DeathState>,
            Without<GridDormant>,
        )>()
        .iter(world)
        .collect();
    for creature in creatures {
        dispatch(world, creature, Hook::UpdateAi(diff_ms));
    }
}

/// The head of `Creature::Update`: `JustAppeared` once after AddToMap or Respawn.
/// `Map::Update` updates objects before relocation notifies, so this precedes the
/// tick's sight and aggro.
type AppearingCreatures = QueryState<
    Entity,
    (
        With<TriggerJustAppeared>,
        With<CreatureAiState>,
        Without<DeathState>,
        Without<GridDormant>,
    ),
>;

fn trigger_just_appeared(world: &mut World, appearing: &mut AppearingCreatures) {
    let creatures: Vec<Entity> = appearing.iter(world).collect();
    for creature in creatures {
        // An earlier creature's hook may have despawned this one.
        let Ok(mut entity) = world.get_entity_mut(creature) else {
            continue;
        };
        entity.remove::<TriggerJustAppeared>();
        dispatch(world, creature, Hook::JustAppeared);
    }
}

/// Run `hook` on `creature`'s AI, or queue it while that AI is running.
pub(crate) fn dispatch(world: &mut World, creature: Entity, hook: Hook) {
    let Some(mut state) = world.get_mut::<CreatureAiState>(creature) else {
        return;
    };
    let Some(mut ai) = state.ai.take() else {
        state.pending.push_back(hook);
        return;
    };
    let mut next = Some(hook);
    while let Some(hook) = next {
        run_hook(world, creature, ai.as_mut(), hook);
        next = world
            .get_mut::<CreatureAiState>(creature)
            .and_then(|mut state| state.pending.pop_front());
    }
    if let Some(mut state) = world.get_mut::<CreatureAiState>(creature) {
        state.ai = Some(ai);
    }
}

/// Source callbacks, separate from NPC eligibility and option validation. These
/// do not bypass the interaction system's authored flags or menu options.
pub(crate) fn creature_gossip_hello(world: &mut World, player: Entity, creature: Entity) -> bool {
    call_gossip(world, creature, |ai, ctx| ai.gossip_hello(ctx, player))
}

pub(crate) fn creature_gossip_select(world: &mut World, player: Entity, creature: Entity) -> bool {
    call_gossip(world, creature, |ai, ctx| ai.gossip_select(ctx, player))
}

fn call_gossip(
    world: &mut World,
    creature: Entity,
    callback: impl FnOnce(&mut dyn CreatureAi, &mut Ctx) -> bool,
) -> bool {
    let Some(mut state) = world.get_mut::<CreatureAiState>(creature) else {
        return false;
    };
    let Some(mut ai) = state.ai.take() else {
        warn!(
            ?creature,
            "Cannot synchronously invoke gossip while creature AI is running"
        );
        return false;
    };
    let handled = callback(
        ai.as_mut(),
        &mut Ctx {
            world,
            me: creature,
        },
    );
    while let Some(hook) = world
        .get_mut::<CreatureAiState>(creature)
        .and_then(|mut state| state.pending.pop_front())
    {
        run_hook(world, creature, ai.as_mut(), hook);
    }
    if let Some(mut state) = world.get_mut::<CreatureAiState>(creature) {
        state.ai = Some(ai);
    }
    handled
}

fn run_hook(world: &mut World, me: Entity, ai: &mut dyn CreatureAi, hook: Hook) {
    if world.get_entity(me).is_err() {
        return;
    }
    let mut ctx = Ctx { world, me };
    match hook {
        Hook::InitializeAi => ai.initialize_ai(&mut ctx),
        Hook::Reset => ai.reset(&mut ctx),
        Hook::JustAppeared => ai.just_appeared(&mut ctx),
        Hook::ChannelFinished { spell_id } => ai.channel_finished(&mut ctx, spell_id),
        Hook::SpellCast { spell_id } => ai.on_spell_cast(&mut ctx, spell_id),
        Hook::JustEngagedWith(who) => ai.just_engaged_with(&mut ctx, who),
        Hook::KilledUnit(victim) => ai.killed_unit(&mut ctx, victim),
        Hook::JustDied(killer) => ai.just_died(&mut ctx, killer),
        Hook::DamageTaken {
            attacker,
            health_before,
            damage,
        } => {
            let max_health = ctx.world.get::<Health>(me).map_or(0.0, |health| health.max);
            ai.damage_taken(
                &mut ctx,
                attacker,
                DamageTaken {
                    health_before,
                    max_health,
                    damage,
                },
            );
        }
        Hook::EnterEvadeMode => ai.enter_evade_mode(&mut ctx),
        Hook::JustReachedHome => ai.just_reached_home(&mut ctx),
        Hook::JustSummoned(summon) => ai.just_summoned(&mut ctx, summon),
        Hook::IsSummonedBy(summoner) => ai.is_summoned_by(&mut ctx, summoner),
        Hook::SummonedCreatureDespawn(summon) => ai.summoned_creature_despawn(&mut ctx, summon),
        Hook::MovementInform(kind, id) => ai.movement_inform(&mut ctx, kind, id),
        Hook::WaypointPathEnded { node_id, path_id } => {
            ai.waypoint_path_ended(&mut ctx, node_id, path_id)
        }
        Hook::DoAction(action) => ai.do_action(&mut ctx, action),
        Hook::StoreCounter { id, value, reset } => ai.store_counter(&mut ctx, id, value, reset),
        Hook::UpdateAi(diff) => ai.update_ai(&mut ctx, diff),
        Hook::QuestAccepted { player, quest_id } => ai.quest_accepted(&mut ctx, player, quest_id),
        Hook::QuestRewarded { player, quest_id } => ai.quest_rewarded(&mut ctx, player, quest_id),
        Hook::SpellHit { caster, spell_id } => ai.spell_hit(&mut ctx, caster, spell_id),
    }
}

/// Pure source-script interception, separate from queued reaction hooks. It must
/// execute before committing health so lethal interception cannot produce death.
pub(crate) fn limit_scripted_loss(
    name: Option<&CreatureScriptName>,
    herald_unkillable: bool,
    health: f32,
    loss: f32,
) -> f32 {
    let animated_gold = name.is_some_and(|name| name.0 == kings_rest::ANIMATED_GOLD_SCRIPT);
    let protected = herald_unkillable || animated_gold;
    if protected && loss >= health {
        (health - 1.0).max(0.0)
    } else {
        loss
    }
}

/// `Unit::DealDamage` → `DamageTaken` for a scripted creature.
pub(crate) fn damage_taken(
    world: &mut World,
    creature: Entity,
    attacker: Option<Entity>,
    health_before: f32,
    damage: f32,
) {
    if world.get::<CreatureAiState>(creature).is_some() {
        dispatch(
            world,
            creature,
            Hook::DamageTaken {
                attacker,
                health_before,
                damage,
            },
        );
    }
}

/// `Spell::TargetInfo::DoDamageAndTriggers` → `CreatureAI::SpellHit` (Spell.cpp:3013-3018)
/// for a scripted creature a spell landed on.
pub(crate) fn spell_hit(world: &mut World, creature: Entity, caster: Entity, spell_id: u32) {
    if world.get::<CreatureAiState>(creature).is_some() {
        dispatch(world, creature, Hook::SpellHit { caster, spell_id });
    }
}

/// TC Spell.cpp:3920–3923: successful cast, before immediate/delayed resolution.
pub(crate) fn on_spell_cast(world: &mut World, caster: Entity, spell_id: u32) {
    if world.get::<CreatureAiState>(caster).is_some() {
        dispatch(world, caster, Hook::SpellCast { spell_id });
    }
}

/// The creature's world and its own entity: the `ScriptedAI` helpers
/// (`me->...`, `DoCast*`, `Talk`, `SelectTarget`).
pub struct Ctx<'w> {
    pub world: &'w mut World,
    pub me: Entity,
}

impl Ctx<'_> {
    pub fn is_alive(&self) -> bool {
        crate::spell_cast::targets::is_alive(self.world, self.me)
            && self.world.get::<DeathState>(self.me).is_none()
    }

    /// `Unit::GetVictim`: the unit the creature chases and attacks.
    pub fn victim(&self) -> Option<Entity> {
        match self.world.get::<CreatureState>(self.me) {
            Some(CreatureState::Pursuing { target }) => Entity::try_from_bits(*target),
            _ => None,
        }
    }

    /// `CreatureAI::IsEngaged`.
    pub fn is_engaged(&self) -> bool {
        self.is_alive()
            && matches!(
                self.world.get::<CreatureState>(self.me),
                Some(CreatureState::Pursuing { .. })
            )
    }

    /// `UNIT_STATE_CASTING`: a cast bar or channel is running.
    pub fn is_casting(&self) -> bool {
        self.world.get::<CastState>(self.me).is_some()
    }

    pub fn react_state(&self) -> ReactState {
        self.world
            .get::<ReactState>(self.me)
            .copied()
            .unwrap_or_default()
    }

    /// `Creature::SetReactState`.
    pub fn set_react_state(&mut self, state: ReactState) {
        self.world.entity_mut(self.me).insert(state);
    }

    /// `CreatureAI::UpdateVictim`: engaged and alive; a passive creature stops
    /// attacking (`AttackStop`). Victim selection and the no-hostiles evade run in
    /// `creature_aggro::update_victim_system`.
    pub fn update_victim(&mut self) -> bool {
        if !self.is_engaged() {
            return false;
        }
        if self.react_state() == ReactState::Passive {
            attack_stop(self.world, self.me);
            return true;
        }
        self.victim().is_some()
    }

    /// `CreatureAI::Talk` → `CreatureTextMgr::SendChat`.
    pub fn talk(&mut self, group: u8) {
        creature_text::talk(self.world, self.me, group);
    }

    /// `UnitAI::DoCast(victim, spellId, triggered)`. True for an accepted cast or
    /// `SPELL_FAILED_SPELL_IN_PROGRESS`, neither of which resumes SmartAI chase.
    pub fn do_cast_at(&mut self, target: Option<Entity>, spell_id: u32, triggered: bool) -> bool {
        if !triggered && self.is_casting() {
            return true;
        }
        let name = self
            .world
            .get::<shared::components::Npc>(self.me)
            .map_or_else(String::new, |npc| npc.name.clone());
        let result =
            crate::spell_cast::creature_cast(self.world, self.me, spell_id, target, triggered);
        #[cfg(test)]
        tests::cast_admission::record(self.world, self.me, spell_id, &result);
        match result {
            Ok(()) => {
                info!("{name} ({}) casts {spell_id} at {target:?}", self.me);
                true
            }
            Err(failure) => {
                debug!("{name} ({}) cast {spell_id} failed: {failure:?}", self.me);
                failure == crate::spell_cast::CastFailure::InProgress
            }
        }
    }

    /// `UnitAI::DoCastVictim`.
    pub fn do_cast_victim(&mut self, spell_id: u32) {
        if let Some(victim) = self.victim() {
            self.do_cast_at(Some(victim), spell_id, false);
        }
    }

    /// `UnitAI::DoCastSelf`.
    pub fn do_cast_self(&mut self, spell_id: u32) {
        let me = self.me;
        self.do_cast_at(Some(me), spell_id, false);
    }

    /// `UnitAI::DoCastAOE`: no explicit target (a creature's spell needing a unit
    /// target takes its victim, `Spell::InitExplicitTargets`).
    pub fn do_cast_aoe(&mut self, spell_id: u32) {
        self.do_cast_at(None, spell_id, false);
    }

    /// `UnitAI::DoCast(spellId)`: the target `UnitAI::FillAISpellInfo` derives
    /// from the spell's first targets.
    pub fn do_cast(&mut self, spell_id: u32) {
        let target = match crate::spell_cast::ai_spell_target(self.world, spell_id) {
            crate::spell_cast::AiTarget::SelfTarget => Some(self.me),
            crate::spell_cast::AiTarget::Victim | crate::spell_cast::AiTarget::Debuff => {
                self.victim()
            }
            crate::spell_cast::AiTarget::Enemy => self.select_target_random(0.0, false),
        };
        if target.is_some() {
            self.do_cast_at(target, spell_id, false);
        }
    }

    /// `UnitAI::SelectTarget(MaxThreat)` with default offset/predicate:
    /// current threat-manager reference first, not numeric top or attack target.
    pub fn select_target_max_threat(&mut self) -> Option<Entity> {
        crate::creature_aggro::current_victim(self.world, self.me)
    }

    /// `UnitAI::SelectTarget(SelectTargetMethod::Random, 0, dist, playerOnly)`: a
    /// random threat-list unit (`DefaultTargetSelector`: within `dist` yards when
    /// `dist` > 0, a player when `player_only`, the current victim included).
    pub fn select_target_random(&self, dist: f32, player_only: bool) -> Option<Entity> {
        let threat = self.world.get::<ThreatTable>(self.me)?;
        let me_pos = crate::spell_cast::targets::position(self.world, self.me)?;
        let candidates: Vec<Entity> = threat
            .entries()
            .iter()
            .filter_map(|entry| Entity::try_from_bits(entry.entity))
            .filter(|&unit| {
                crate::spell_cast::targets::is_alive(self.world, unit)
                    && (!player_only || self.world.get::<Player>(unit).is_some())
                    && (dist <= 0.0
                        || crate::spell_cast::targets::position(self.world, unit)
                            .is_some_and(|pos| pos.distance(me_pos) <= dist))
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = rand::Rng::gen_range(&mut rand::thread_rng(), 0..candidates.len());
        Some(candidates[pick])
    }

    /// `SelectTarget(SelectTargetMethod::Random, 0, NonTankTargetSelector(me))`
    /// (UnitAICommon.cpp): a random living threat-list player other than the
    /// current victim.
    pub fn select_target_random_non_tank(&self) -> Option<Entity> {
        let victim = self.victim();
        let threat = self.world.get::<ThreatTable>(self.me)?;
        let candidates: Vec<Entity> = threat
            .entries()
            .iter()
            .filter_map(|entry| Entity::try_from_bits(entry.entity))
            .filter(|&unit| {
                Some(unit) != victim
                    && self.world.get::<Player>(unit).is_some()
                    && crate::spell_cast::targets::is_alive(self.world, unit)
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = rand::Rng::gen_range(&mut rand::thread_rng(), 0..candidates.len());
        Some(candidates[pick])
    }

    /// `FindNearestCreatureWithOptions(range, { .StringId = id })` in this copy.
    /// Source leaves IsAlive unset; DeathState here also marks respawn-pending
    /// actors, so dead creatures are excluded like the Ragefire boss lookups.
    pub fn find_nearest_creature_with_string_id(&mut self, range: f32, id: &str) -> Option<Entity> {
        let origin = crate::spell_cast::targets::position(self.world, self.me)?;
        let map = self.map()?;
        let me = self.me;
        self.world
            .query_filtered::<(
                Entity,
                &InMap,
                &crate::zones::CreatureStringIds,
                &shared::components::Position,
            ), Without<DeathState>>()
            .iter(self.world)
            .filter(|(entity, copy, ids, _)| *entity != me && **copy == map && ids.has(id))
            .map(|(entity, _, _, position)| {
                let position = Vec3::new(position.x, position.y, position.z);
                (entity, position.distance(origin))
            })
            .filter(|(_, distance)| *distance <= range)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity)
    }

    /// `Unit::RemoveAllAuras`.
    pub fn remove_all_auras(&mut self) {
        crate::auras::remove_auras(
            self.world,
            self.me,
            crate::auras::RemoveReason::Cancelled,
            |_| true,
        );
    }

    /// `WorldObject::SummonCreature(entry, pos)` (TEMPSUMMON_MANUAL_DESPAWN) at a
    /// WoW position `[x, y, z, o]`; the summoner's `JustSummoned` follows.
    pub fn summon_creature(&mut self, entry: u32, position: [f32; 4]) -> Option<Entity> {
        crate::zones::summon_creature(self.world, self.me, entry, position)
    }

    /// `WorldObject::SummonCreatureGroup`: authored membership, positions and lifetimes.
    pub fn summon_creature_group(&mut self, group_id: u32) -> Option<Vec<Entity>> {
        smart_ai::summons::summon_creature_group(self, group_id)
    }

    /// `MotionMaster::MovePoint(id, pos)` of `unit`, walking when it walks.
    pub fn move_point(&mut self, unit: Entity, id: u32, target: shared::components::Position) {
        movement::move_point(self.world, unit, id, target);
    }

    /// `Unit::SetWalk`.
    pub fn set_walk(&mut self, unit: Entity, walk: bool) {
        movement::set_walk(self.world, unit, walk);
    }

    pub fn position(&self) -> Option<shared::components::Position> {
        self.world
            .get::<shared::components::Position>(self.me)
            .copied()
    }

    /// `Player::KilledMonsterCredit(entry)` for each player tapping the creature
    /// (`SMART_ACTION_CALL_KILLEDMONSTER` without a target, SmartScript.cpp:928-942).
    pub fn credit_tappers(&mut self, entry: u32) {
        let tappers = self
            .world
            .get::<crate::creature_tap::CreatureTap>(self.me)
            .map_or_else(Vec::new, |tap| tap.tappers.clone());
        let mut players = self
            .world
            .query_filtered::<(Entity, &crate::persistence::ClientId), With<Player>>();
        let tapping: Vec<Entity> = players
            .iter(self.world)
            .filter(|(_, id)| tappers.contains(&id.0))
            .map(|(player, _)| player)
            .collect();
        for player in tapping {
            crate::quest::killed_monster_credit(self.world, player, entry);
        }
    }

    /// `Unit::KillSelf`.
    pub fn kill_self(&mut self) {
        crate::combat::kill_unit(self.world, self.me, self.me);
    }

    /// `CreatureAI::DoZoneInCombat(creature)`: every living player of the map
    /// engages `creature`.
    pub fn do_zone_in_combat(&mut self, creature: Entity) {
        let Some(map) = self.world.get::<InMap>(creature).copied() else {
            return;
        };
        for player in instance_script::players_in_map(self.world, map) {
            if crate::spell_cast::targets::is_alive(self.world, player) {
                crate::creature_aggro::add_threat(self.world, creature, player, 0.0);
            }
        }
    }

    /// The map copy's `InstanceScript` (`me->GetInstanceScript()`).
    pub fn map(&self) -> Option<InMap> {
        self.world.get::<InMap>(self.me).copied()
    }

    /// `DUNGEON_MODE(normal, heroic)`: the Heroic value in a difficulty2 copy.
    pub fn dungeon_mode(&self, normal: u32, heroic: u32) -> u32 {
        let map = self.map().expect("dungeon script actor map");
        if crate::zones::map_difficulty_id(self.world, map) == 2 {
            heroic
        } else {
            normal
        }
    }
}

/// `Unit::AttackStop`: stop swinging at the victim.
pub(crate) fn attack_stop(world: &mut World, unit: Entity) {
    if let Some(mut target) = world.get_mut::<CombatTarget>(unit)
        && target.0.is_some()
    {
        target.0 = None;
    }
    crate::combat::attack_stop(world, unit);
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
#[test]
fn idle_just_appeared_full_population_is_bounded() {
    let mut app = App::new();
    app.add_systems(Update, trigger_just_appeared);
    crate::test_data::populate_idle_world(app.world_mut(), || crate::grid_activity::GridDormant);
    crate::test_data::assert_idle_updates_are_bounded(
        &mut app,
        "idle_just_appeared_full_population_is_bounded",
    );
}
