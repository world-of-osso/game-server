//! TrinityCore `SmartAI` (game/AI/SmartScripts/) for creatures whose `AIName` is
//! `SmartAI`, driven by their TDB `smart_scripts` rows (`source_type` 0, rows keyed by
//! entry; rows of one spawn guid are not loaded).
//!
//! `SmartScript` keeps an event phase and per event a timer, `active` and `runOnce`.
//! `OnReset` (spawn, respawn, evade) re-inits the timers and runs `RESET` events;
//! `OnUpdate` ticks the timed events; hooks (`SpellHit`) run the events of their type
//! (`ProcessEventsFor`). An event runs when it is active, in phase (`event_phase_mask`
//! 0 = every phase) and not a spent `NOT_REPEATABLE` one; its action then runs, and its
//! `link` event right after with the same invoker.
//!
//! Quest accept/reward can invoke timed action lists and authored TDB waypoint
//! paths. Reached/pause/resume events, stored invokers and quest event credit are
//! interpreted generically. Unsupported event/action/target kinds are diagnosed
//! once per kind at startup, including rows outside the loaded creature subset.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use rand::Rng;
use rusqlite::{Connection, OpenFlags};

use super::{CreatureAi, Ctx};

#[path = "smart_paths.rs"]
mod paths;

#[path = "smart_timed.rs"]
mod timed;

#[path = "smart_instance_data.rs"]
mod instance_data;

#[path = "smart_combat.rs"]
mod combat;
#[path = "smart_escort.rs"]
mod escort;
#[path = "smart_pose.rs"]
mod pose;

#[path = "smart_targets.rs"]
mod targets;

#[path = "smart_lifetimes.rs"]
pub(super) mod lifetimes;

#[path = "smart_summons.rs"]
pub(super) mod summons;

#[path = "smart_follow.rs"]
mod follow;

#[cfg(test)]
#[path = "smart_escort_tests.rs"]
mod escort_tests;

// `SMART_EVENT` (SmartScriptMgr.h).
const EVENT_UPDATE_IC: u32 = 0;
const EVENT_FOLLOW_COMPLETED: u32 = 65;
const EVENT_AGGRO: u32 = 4;
const EVENT_DEATH: u32 = 6;
const EVENT_EVADE: u32 = 7;
const EVENT_SUMMONED_UNIT: u32 = 17;
const EVENT_SUMMON_DESPAWNED: u32 = 35;
const EVENT_JUST_SUMMONED: u32 = 54;
const EVENT_SPELLHIT: u32 = 8;
const EVENT_RANGE: u32 = 9;
const EVENT_HAS_AURA: u32 = 23;
const EVENT_RESET: u32 = 25;
const EVENT_RESPAWN: u32 = 11;
const RESPAWN_CONDITION_MAP: u32 = 1;
const RESPAWN_CONDITION_AREA: u32 = 2;
const EVENT_LINK: u32 = 61;
const EVENT_ACCEPTED_QUEST: u32 = 19;
const EVENT_REWARD_QUEST: u32 = 20;
const EVENT_WAYPOINT_REACHED: u32 = 40;
const EVENT_WAYPOINT_PAUSED: u32 = 55;
const EVENT_WAYPOINT_RESUMED: u32 = 56;
const EVENT_WAYPOINT_STOPPED: u32 = 57;
const EVENT_WAYPOINT_ENDED: u32 = 58;
const EVENT_COUNTER_SET: u32 = 77;
// `SMART_ACTION`.
const ACTION_TALK: u32 = 1;
const ACTION_SET_EMOTE_STATE: u32 = 17;
const ACTION_SET_ORIENTATION: u32 = 66;
const ACTION_SET_FACTION: u32 = 2;
const ACTION_FAIL_QUEST: u32 = 6;
const ACTION_COMPLETE_QUEST: u32 = 152;
const ACTION_CAST: u32 = 11;
const ACTION_SUMMON_CREATURE: u32 = 12;
const ACTION_SUMMON_CREATURE_GROUP: u32 = 107;
const ACTION_FOLLOW: u32 = 29;
const ACTION_SET_EVENT_PHASE: u32 = 22;
const ACTION_CALL_KILLEDMONSTER: u32 = 33;
const ACTION_SET_INST_DATA: u32 = 34;
const ACTION_DIE: u32 = 37;
const ACTION_ATTACK_START: u32 = 49;
const ACTION_SET_IMMUNE_NPC: u32 = 145;
const ACTION_FORCE_DESPAWN: u32 = 41;
const ACTION_WP_START: u32 = 53;
const ACTION_WP_PAUSE: u32 = 54;
const ACTION_WP_STOP: u32 = 55;
const ACTION_SET_RUN: u32 = 59;
const ACTION_SET_NPC_FLAG: u32 = 81;
const ACTION_ADD_NPC_FLAG: u32 = 82;
const ACTION_WP_RESUME: u32 = 65;
const ACTION_SET_COUNTER: u32 = 63;
const ACTION_STORE_TARGET_LIST: u32 = 64;
const ACTION_CALL_TIMED_ACTION_LIST: u32 = 80;
const ACTION_REMOVE_NPC_FLAG: u32 = 83;
// `SMART_TARGET`.
const TARGET_NONE: u32 = 0;
const TARGET_SELF: u32 = 1;
const TARGET_VICTIM: u32 = 2;
const TARGET_HOSTILE_RANDOM: u32 = 5;
const TARGET_ACTION_INVOKER: u32 = 7;
const TARGET_STORED: u32 = 12;
const TARGET_INVOKER_PARTY: u32 = 16;
const TARGET_CLOSEST_CREATURE: u32 = 19;
const TARGET_POSITION: u32 = 8;
// `SMART_EVENT_FLAG_*`, `SMARTCAST_*`.
const EVENT_FLAG_NOT_REPEATABLE: u32 = 0x001;
const EVENT_FLAG_DONT_RESET: u32 = 0x100;
const CAST_TRIGGERED: u32 = 0x02;
const CAST_AURA_NOT_PRESENT: u32 = 0x20;
const CAST_COMBAT_MOVE: u32 = 0x40;

/// `SmartAI::_canCombatMove == false`: hold chase between successful CMC casts.
#[derive(Component)]
pub(crate) struct CombatMovementDisabled;

/// One `smart_scripts` row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SmartEvent {
    pub id: u32,
    pub link: u32,
    pub event_type: u32,
    pub phase_mask: u32,
    pub chance: u32,
    pub flags: u32,
    pub event_params: [u32; 5],
    pub action_type: u32,
    pub action_params: [u32; 7],
    pub target_type: u32,
    pub target_params: [u32; 4],
    pub target_position: [f32; 4],
}

/// Creature entry -> its SmartAI rows, by id.
#[derive(Resource, Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct SmartScripts(
    pub HashMap<u32, Arc<Vec<SmartEvent>>>,
    HashMap<u32, Arc<Vec<SmartEvent>>>,
    HashMap<u32, Arc<paths::WaypointPath>>,
    HashMap<(u32, u32), Vec<summons::GroupMember>>,
);

const SMART_QUERY: &str = "SELECT s.entryorguid, s.id, s.link, s.event_type, s.event_phase_mask, \
     s.event_chance, s.event_flags, s.event_param1, s.event_param2, s.event_param3, s.event_param4, \
     s.event_param5, s.action_type, s.action_param1, s.action_param2, s.action_param3, \
     s.action_param4, s.action_param5, s.action_param6, s.action_param7, s.target_type, \
     s.source_type, s.target_param1, s.target_param2, s.target_param3, s.target_param4, \
     s.target_x, s.target_y, s.target_z, s.target_o FROM tdb_smart_scripts s \
     WHERE s.source_type = 9 OR (s.source_type = 0 AND s.entryorguid > 0 AND EXISTS \
     (SELECT 1 FROM content_creature_template t WHERE t.entry=s.entryorguid AND t.AIName='SmartAI')) \
     ORDER BY s.entryorguid, s.id";

pub(crate) fn load_smart_scripts(path: &Path) -> rusqlite::Result<SmartScripts> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    log_unsupported_types(&conn)?;
    let mut statement = conn.prepare(SMART_QUERY)?;
    let rows = statement.query_map([], |row| {
        let int = |index: usize| row.get::<_, u32>(index);
        Ok((
            int(0)?,
            int(21)?,
            SmartEvent {
                id: int(1)?,
                link: int(2)?,
                event_type: int(3)?,
                phase_mask: int(4)?,
                chance: int(5)?,
                flags: int(6)?,
                event_params: [int(7)?, int(8)?, int(9)?, int(10)?, int(11)?],
                action_type: int(12)?,
                action_params: [
                    int(13)?,
                    int(14)?,
                    int(15)?,
                    int(16)?,
                    int(17)?,
                    int(18)?,
                    int(19)?,
                ],
                target_type: int(20)?,
                target_params: [int(22)?, int(23)?, int(24)?, int(25)?],
                target_position: [row.get(26)?, row.get(27)?, row.get(28)?, row.get(29)?],
            },
        ))
    })?;
    let mut scripts: HashMap<u32, Vec<SmartEvent>> = HashMap::new();
    let mut lists: HashMap<u32, Vec<SmartEvent>> = HashMap::new();
    for row in rows {
        let (entry, source, event) = row?;
        let destination = if source == 9 {
            &mut lists
        } else {
            &mut scripts
        };
        destination.entry(entry).or_default().push(event);
    }
    info!(
        "Loaded SmartAI scripts for {} creatures and {} timed lists",
        scripts.len(),
        lists.len()
    );
    Ok(SmartScripts(
        scripts
            .into_iter()
            .map(|(entry, events)| (entry, Arc::new(events)))
            .collect(),
        lists
            .into_iter()
            .map(|(entry, events)| (entry, Arc::new(events)))
            .collect(),
        paths::load_waypoint_paths(&conn)?,
        summons::load_summon_groups(&conn)?,
    ))
}

fn supports_event(kind: u32) -> bool {
    matches!(
        kind,
        EVENT_UPDATE_IC
            | EVENT_RANGE
            | EVENT_AGGRO
            | EVENT_DEATH
            | EVENT_FOLLOW_COMPLETED
            | EVENT_SUMMONED_UNIT
            | EVENT_SUMMON_DESPAWNED
            | EVENT_JUST_SUMMONED
            | EVENT_EVADE
            | EVENT_SPELLHIT
            | EVENT_HAS_AURA
            | EVENT_RESET
            | EVENT_RESPAWN
            | EVENT_LINK
            | EVENT_ACCEPTED_QUEST
            | EVENT_REWARD_QUEST
            | EVENT_WAYPOINT_REACHED
            | EVENT_WAYPOINT_PAUSED
            | EVENT_WAYPOINT_RESUMED
            | EVENT_WAYPOINT_STOPPED
            | EVENT_WAYPOINT_ENDED
            | EVENT_COUNTER_SET
    )
}

fn supports_action(kind: u32) -> bool {
    matches!(
        kind,
        ACTION_TALK
            | ACTION_CAST
            | ACTION_SUMMON_CREATURE
            | ACTION_SUMMON_CREATURE_GROUP
            | ACTION_FOLLOW
            | ACTION_SET_EMOTE_STATE
            | ACTION_SET_ORIENTATION
            | ACTION_SET_EVENT_PHASE
            | ACTION_CALL_KILLEDMONSTER
            | ACTION_SET_INST_DATA
            | ACTION_DIE
            | ACTION_ATTACK_START
            | ACTION_SET_IMMUNE_NPC
            | ACTION_FORCE_DESPAWN
            | ACTION_FAIL_QUEST
            | ACTION_COMPLETE_QUEST
            | ACTION_WP_START
            | ACTION_WP_PAUSE
            | ACTION_WP_RESUME
            | ACTION_STORE_TARGET_LIST
            | ACTION_SET_COUNTER
            | ACTION_CALL_TIMED_ACTION_LIST
            | ACTION_REMOVE_NPC_FLAG
            | ACTION_SET_FACTION
            | ACTION_WP_STOP
            | ACTION_SET_RUN
            | ACTION_SET_NPC_FLAG
            | ACTION_ADD_NPC_FLAG
    )
}

fn supports_target(kind: u32) -> bool {
    matches!(
        kind,
        TARGET_NONE
            | TARGET_SELF
            | TARGET_VICTIM
            | TARGET_HOSTILE_RANDOM
            | TARGET_ACTION_INVOKER
            | TARGET_STORED
            | TARGET_INVOKER_PARTY
            | TARGET_CLOSEST_CREATURE
            | TARGET_POSITION
    )
}

fn log_unsupported_types(conn: &Connection) -> rusqlite::Result<()> {
    let mut query = conn.prepare("SELECT event_type,action_type,target_type,COUNT(*) FROM tdb_smart_scripts GROUP BY event_type,action_type,target_type")?;
    let rows = query.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u64>(3)?,
        ))
    })?;
    let mut unsupported = std::collections::BTreeMap::new();
    for row in rows {
        let (event, action, target, count) = row?;
        for (dimension, kind, supported) in [
            ("event", event, supports_event(event)),
            ("action", action, supports_action(action)),
            ("target", target, supports_target(target)),
        ] {
            if !supported {
                *unsupported.entry((dimension, kind)).or_insert(0_u64) += count;
            }
        }
    }
    for ((dimension, kind), count) in unsupported {
        warn!("SmartAI unsupported {dimension} type {kind}: {count} TDB rows");
    }
    Ok(())
}

pub(crate) fn startup_load_smart_scripts(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    if !path.exists() {
        warn!("world.db not found, no SmartAI scripts");
        commands.insert_resource(SmartScripts::default());
        return;
    }
    let scripts = load_smart_scripts(path)
        .unwrap_or_else(|error| panic!("load tdb_smart_scripts from {}: {error}", path.display()));
    commands.insert_resource(scripts);
}

/// Whether `entry` runs SmartAI.
pub(crate) fn has_script(world: &World, entry: u32) -> bool {
    world
        .get_resource::<SmartScripts>()
        .is_some_and(|scripts| scripts.0.contains_key(&entry))
}

/// `SmartScriptHolder`'s runtime state.
#[derive(Debug, Clone, Default)]
struct EventState {
    timer_ms: u32,
    active: bool,
    run_once: bool,
    ignore_chance_roll: bool,
}

pub(crate) struct SmartAi {
    events: Arc<Vec<SmartEvent>>,
    states: Vec<EventState>,
    phase: u32,
    boss: Option<super::boss_ai::BossAi>,
    stored_targets: HashMap<u32, Vec<Entity>>,
    counters: HashMap<u32, u32>,
    timed_list: Option<TimedList>,
    processing_timed_list: bool,
    escort: Option<paths::EscortPath>,
    path_event_id: Option<u32>,
    follow: Option<follow::Follow>,
}

struct TimedList {
    events: Arc<Vec<SmartEvent>>,
    index: usize,
    timer_ms: u32,
    invoker: Option<Entity>,
    timer_type: u32,
}

/// The SmartAI of a creature of `entry`, if it has rows.
pub(crate) fn creature_ai(world: &World, creature: Entity) -> Option<Box<dyn CreatureAi>> {
    let entry = world.get::<shared::components::Npc>(creature)?.template_id;
    let boss = super::instance_script::bound_boss(world, creature);
    let rows = world
        .get_resource::<SmartScripts>()
        .and_then(|scripts| scripts.0.get(&entry));
    if rows.is_none() && boss.is_none() {
        return None;
    }
    // A bound boss without authored rows intentionally has lifecycle and melee only.
    let events = rows.cloned().unwrap_or_default();
    let states = vec![EventState::default(); events.len()];
    let mut ai = SmartAi {
        events,
        states,
        phase: 0,
        boss,
        stored_targets: HashMap::new(),
        counters: HashMap::new(),
        timed_list: None,
        processing_timed_list: false,
        escort: None,
        path_event_id: None,
        follow: None,
    };
    // TC OnInitialize initializes holders without firing RESET events. Appearance
    // then runs RESPAWN before OnReset (SmartAI.cpp:494–519).
    for index in 0..ai.events.len() {
        ai.init_timer(index);
    }
    Some(Box::new(ai))
}

impl SmartAi {
    /// `SmartScript::ProcessAction(SMART_ACTION_CAST)`, SmartScript.cpp:565-637.
    /// Failed CMC casts restore combat movement; accepted casts stop it.
    fn cast_targets(ctx: &mut Ctx, event: &SmartEvent, targets: Vec<Entity>) -> bool {
        let [spell, cast_flags, ..] = event.action_params;
        let mut failed = false;
        let mut successful = false;
        for target in targets {
            if cast_flags & CAST_AURA_NOT_PRESENT != 0
                && crate::auras::aura_count(ctx.world, target, spell) > 0
            {
                continue;
            }
            let accepted = ctx.do_cast_at(Some(target), spell, cast_flags & CAST_TRIGGERED != 0);
            if cast_flags & CAST_COMBAT_MOVE != 0 {
                if accepted {
                    ctx.world.entity_mut(ctx.me).insert(CombatMovementDisabled);
                } else {
                    ctx.world
                        .entity_mut(ctx.me)
                        .remove::<CombatMovementDisabled>();
                }
            }
            successful |= accepted;
            failed |= !accepted;
        }
        !failed || successful
    }

    /// `SmartScript::IsInPhase`.
    fn in_phase(&self, mask: u32) -> bool {
        mask == 0 || (self.phase != 0 && (1 << (self.phase - 1)) & mask != 0)
    }

    /// `SmartScript::InitTimer`: combat updates use their initial min/max.
    fn init_timer(&mut self, index: usize) {
        self.states[index] = EventState::default();
        let event = &self.events[index];
        let (min, max) = if event.event_type == EVENT_UPDATE_IC {
            (event.event_params[0], event.event_params[1])
        } else {
            (0, 0)
        };
        self.recalc_timer(index, min, max);
    }

    /// `SmartScript::RecalcTimer`.
    fn recalc_timer(&mut self, index: usize, min: u32, max: u32) {
        let timer = if max > min {
            rand::thread_rng().gen_range(min..=max)
        } else {
            min
        };
        self.states[index].timer_ms = timer;
        self.states[index].active = timer == 0;
    }

    /// `SmartScript::ProcessEventsFor(type)`; link events run only through their source.
    fn process_events_for(
        &mut self,
        ctx: &mut Ctx,
        event_type: u32,
        invoker: Option<Entity>,
        spell: u32,
    ) {
        for index in 0..self.events.len() {
            if self.events[index].event_type == event_type && event_type != EVENT_LINK {
                self.process_event(ctx, index, invoker, spell);
            }
        }
    }

    /// `SmartScript::ProcessEvent` (SmartScript.cpp:3176-3395) for the modelled types.
    fn process_event(&mut self, ctx: &mut Ctx, index: usize, invoker: Option<Entity>, spell: u32) {
        let event = self.events[index].clone();
        if !self.states[index].active && event.event_type != EVENT_LINK {
            return;
        }
        if !self.in_phase(event.phase_mask)
            || (event.flags & EVENT_FLAG_NOT_REPEATABLE != 0 && self.states[index].run_once)
        {
            return;
        }
        match event.event_type {
            EVENT_LINK
            | EVENT_RESET
            | EVENT_AGGRO
            | EVENT_DEATH
            | EVENT_FOLLOW_COMPLETED
            | EVENT_JUST_SUMMONED
            | EVENT_EVADE => self.process_action(ctx, index, invoker),
            EVENT_RESPAWN => {
                let applies = match event.event_params[0] {
                    RESPAWN_CONDITION_MAP => ctx
                        .map()
                        .is_some_and(|map| map.map_id == event.event_params[1]),
                    RESPAWN_CONDITION_AREA => ctx
                        .world
                        .get::<shared::components::Zone>(ctx.me)
                        .is_some_and(|zone| zone.id == event.event_params[2]),
                    _ => true,
                };
                if applies {
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_RANGE => self.process_range_event(ctx, index),
            EVENT_UPDATE_IC => {
                if ctx.is_engaged() {
                    self.recalc_timer(index, event.event_params[2], event.event_params[3]);
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_SUMMONED_UNIT | EVENT_SUMMON_DESPAWNED => {
                if invoker
                    .and_then(|unit| ctx.world.get::<shared::components::Npc>(unit))
                    .is_some_and(|npc| {
                        event.event_params[0] == 0 || event.event_params[0] == npc.template_id
                    })
                {
                    self.recalc_timer(index, event.event_params[1], event.event_params[2]);
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_COUNTER_SET => {
                let [id, value, cooldown_min, cooldown_max, _] = event.event_params;
                if id == spell && self.counters.get(&id).copied().unwrap_or(0) == value {
                    self.recalc_timer(index, cooldown_min, cooldown_max);
                    self.process_action(ctx, index, None);
                }
            }
            EVENT_ACCEPTED_QUEST | EVENT_REWARD_QUEST => {
                if event.event_params[0] == 0 || event.event_params[0] == spell {
                    self.recalc_timer(index, event.event_params[1], event.event_params[2]);
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_WAYPOINT_REACHED
            | EVENT_WAYPOINT_PAUSED
            | EVENT_WAYPOINT_RESUMED
            | EVENT_WAYPOINT_STOPPED
            | EVENT_WAYPOINT_ENDED => {
                let path_id = self
                    .escort
                    .as_ref()
                    .map(|escort| escort.path_id)
                    .or(self.path_event_id)
                    .unwrap_or(0);
                if (event.event_params[0] == u32::MAX || event.event_params[0] == spell)
                    && (event.event_params[1] == 0 || event.event_params[1] == path_id)
                {
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_SPELLHIT => {
                let [hit_spell, _school, cooldown_min, cooldown_max, _] = event.event_params;
                if hit_spell == 0 || hit_spell == spell {
                    self.recalc_timer(index, cooldown_min, cooldown_max);
                    self.process_action(ctx, index, invoker);
                }
            }
            EVENT_HAS_AURA => {
                let [aura, count, repeat_min, repeat_max, _] = event.event_params;
                let present = crate::auras::aura_count(ctx.world, ctx.me, aura);
                if (count == 0 && present == 0) || (count != 0 && present >= count) {
                    self.recalc_timer(index, repeat_min, repeat_max);
                    self.process_action(ctx, index, invoker);
                }
            }
            _ => {}
        }
    }

    /// TC SmartScript.cpp3226–3235: engaged victim in reach-adjusted3D range;
    /// an excluded victim gets a predictable500ms recheck, not the action repeat.
    fn process_range_event(&mut self, ctx: &mut Ctx, index: usize) {
        if !ctx.is_engaged() {
            return;
        }
        let Some(victim) = ctx.victim() else {
            return;
        };
        let event = &self.events[index];
        let [min, max, repeat_min, repeat_max, ..] = event.event_params;
        let in_range = super::model_reach::distance(ctx.world, ctx.me, victim)
            .is_some_and(|distance| distance >= min as f32 && distance < max as f32);
        if in_range {
            self.recalc_timer(index, repeat_min, repeat_max);
            self.process_action(ctx, index, Some(victim));
        } else {
            self.recalc_timer(index, 500, 500);
        }
    }

    /// `SmartScript::ProcessAction` (SmartScript.cpp): the action, then the link event.
    fn process_action(&mut self, ctx: &mut Ctx, index: usize, invoker: Option<Entity>) {
        let event = self.events[index].clone();
        self.states[index].run_once = true;
        if event.event_type != EVENT_LINK
            && !self.states[index].ignore_chance_roll
            && event.chance > 0
            && event.chance < 100
            && rand::thread_rng().gen_range(0..100) >= event.chance
        {
            return;
        }
        self.states[index].ignore_chance_roll = false;
        if !self.execute_action(ctx, &event, invoker) {
            self.recalc_timer(index, 1, 1);
            self.states[index].run_once = false;
            self.states[index].ignore_chance_roll = true;
            return;
        }
        if event.link != 0 && event.link != event.id {
            if let Some(linked) = self.events.iter().position(|e| e.id == event.link) {
                self.process_event(ctx, linked, invoker, 0);
            }
        }
    }

    fn execute_action(
        &mut self,
        ctx: &mut Ctx,
        event: &SmartEvent,
        invoker: Option<Entity>,
    ) -> bool {
        let targets = match event.target_type {
            TARGET_SELF => vec![ctx.me],
            TARGET_VICTIM => ctx.victim().into_iter().collect(),
            TARGET_HOSTILE_RANDOM => {
                targets::random_hostile(ctx.world, ctx.me, event.target_params)
                    .into_iter()
                    .collect()
            }
            TARGET_ACTION_INVOKER => invoker.into_iter().collect(),
            TARGET_INVOKER_PARTY => {
                invoker.map_or_else(Vec::new, |player| targets::party_targets(ctx.world, player))
            }
            TARGET_CLOSEST_CREATURE => {
                let [entry, distance, alive, ..] = event.target_params;
                let radius = if distance == 0 {
                    100.0
                } else {
                    distance as f32
                };
                targets::nearest_creature(ctx.world, ctx.me, entry, radius, alive)
                    .into_iter()
                    .collect()
            }
            TARGET_STORED => self
                .stored_targets
                .get(&event.target_params[0])
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        match event.action_type {
            ACTION_TALK => {
                let speaker = targets
                    .iter()
                    .copied()
                    .find(|&entity| ctx.world.get::<shared::components::Npc>(entity).is_some())
                    .unwrap_or(ctx.me);
                Ctx {
                    world: ctx.world,
                    me: speaker,
                }
                .talk(event.action_params[0] as u8);
            }
            ACTION_SUMMON_CREATURE => self.summon_action(ctx, event, &targets, invoker),
            ACTION_SUMMON_CREATURE_GROUP => self.summon_group(ctx, event, invoker),
            ACTION_FOLLOW => self.set_follow(ctx, event, &targets),
            ACTION_SET_EMOTE_STATE => Self::set_emote_state(ctx, &targets, event.action_params[0]),
            ACTION_SET_ORIENTATION => Self::set_orientation(ctx, event, &targets),
            ACTION_CAST => {
                return Self::cast_targets(ctx, event, targets);
            }
            ACTION_ATTACK_START => combat::attack_random_target(ctx, &targets),
            ACTION_SET_IMMUNE_NPC => {
                combat::set_npc_immunity(ctx, &targets, event.action_params[0] != 0)
            }
            ACTION_SET_EVENT_PHASE => self.phase = event.action_params[0],
            ACTION_SET_INST_DATA => return instance_data::apply(ctx, event.action_params),
            ACTION_FAIL_QUEST | ACTION_COMPLETE_QUEST => {
                for target in targets {
                    crate::quest::persist_scripted_quest_result(
                        ctx.world,
                        target,
                        event.action_params[0],
                        event.action_type == ACTION_FAIL_QUEST,
                    );
                }
            }
            ACTION_CALL_KILLEDMONSTER => {
                let entry = event.action_params[0];
                if matches!(event.target_type, TARGET_NONE | TARGET_SELF) {
                    ctx.credit_tappers(entry);
                } else {
                    for target in targets {
                        crate::quest::killed_monster_credit(ctx.world, target, entry);
                    }
                }
            }
            ACTION_FORCE_DESPAWN => {
                for target in targets {
                    if ctx.world.get::<shared::components::Npc>(target).is_some() {
                        lifetimes::schedule_despawn(
                            ctx.world,
                            target,
                            event.action_params[0],
                            event.action_params[1],
                            false,
                        );
                    }
                }
            }
            ACTION_DIE => {
                if ctx.is_alive() {
                    ctx.kill_self();
                }
            }
            ACTION_SET_COUNTER => {
                let [id, value, reset, ..] = event.action_params;
                if targets.is_empty() {
                    self.store_counter(ctx, id, value, reset != 0);
                }
                for target in targets {
                    if target == ctx.me {
                        self.store_counter(ctx, id, value, reset != 0);
                    } else {
                        super::dispatch(
                            ctx.world,
                            target,
                            super::Hook::StoreCounter {
                                id,
                                value,
                                reset: reset != 0,
                            },
                        );
                    }
                }
            }
            ACTION_STORE_TARGET_LIST => {
                self.stored_targets.insert(event.action_params[0], targets);
            }
            ACTION_CALL_TIMED_ACTION_LIST => self.start_timed_list(ctx, event, invoker),
            ACTION_SET_FACTION => Self::set_faction(ctx, &targets, event.action_params[0]),
            ACTION_SET_NPC_FLAG | ACTION_ADD_NPC_FLAG | ACTION_REMOVE_NPC_FLAG => {
                Self::change_npc_flags(ctx, &targets, event);
            }
            ACTION_SET_RUN => {
                let walk = event.action_params[0] == 0;
                if let Some(escort) = &mut self.escort {
                    escort.walk = walk;
                }
                ctx.set_walk(ctx.me, walk);
            }
            ACTION_WP_STOP => {
                if let Some(escort) = &mut self.escort {
                    if event.action_params[1] != 0 {
                        escort.quest_id = event.action_params[1];
                    }
                    escort.despawn_ms = event.action_params[0];
                }
                if self.escort.is_none()
                    && event.action_params[2] == 0
                    && event.action_params[0] > 0
                {
                    lifetimes::schedule_despawn(ctx.world, ctx.me, event.action_params[0], 0, true);
                }
                self.stop_path(ctx, event.action_params[2] != 0);
            }
            ACTION_WP_START => self.start_path(ctx, event, invoker, targets),
            ACTION_WP_PAUSE => self.pause_path(ctx, event.action_params[0]),
            ACTION_WP_RESUME => {
                if let Some(escort) = &mut self.escort
                    && escort.pause_ms.is_some()
                {
                    escort.pause_ms = Some(1);
                }
            }
            _ => {}
        }
        true
    }

    fn start_path(
        &mut self,
        ctx: &mut Ctx,
        event: &SmartEvent,
        invoker: Option<Entity>,
        targets: Vec<Entity>,
    ) {
        let path_id = event.action_params[1];
        let path = ctx
            .world
            .resource::<SmartScripts>()
            .2
            .get(&path_id)
            .cloned();
        let Some(path) = path else {
            warn!(path_id, "SmartAI waypoint path is absent");
            return;
        };
        self.escort = Some(paths::EscortPath {
            path_id,
            path: path.clone(),
            index: 0,
            pause_ms: None,
            reached: false,
            walk: path.walk,
            repeat: event.action_params[2] != 0,
            quest_id: event.action_params[3],
            targets: targets
                .into_iter()
                .filter(|&entity| {
                    ctx.world
                        .get::<shared::components::Player>(entity)
                        .is_some()
                })
                .collect(),
            npc_flags: ctx
                .world
                .get::<shared::protocol::NpcFlags>(ctx.me)
                .map_or(0, |flags| flags.0),
            despawn_ms: event.action_params[4],
            node_delay_ms: None,
            range_check_ms: 1000,
        });
        super::movement::retain_path_home(ctx.world, ctx.me);
        self.move_to_waypoint(ctx);
        if invoker.is_some_and(|entity| {
            ctx.world
                .get::<shared::components::Player>(entity)
                .is_some()
        }) {
            if let Some(mut flags) = ctx.world.get_mut::<shared::protocol::NpcFlags>(ctx.me) {
                flags.0 = 0;
            }
        }
    }

    fn set_faction(ctx: &mut Ctx, targets: &[Entity], requested: u32) {
        for &target in targets {
            let faction = if requested == 0 {
                let Some(entry) = ctx
                    .world
                    .get::<shared::components::Npc>(target)
                    .map(|npc| npc.template_id)
                else {
                    continue;
                };
                let Some(template) = ctx
                    .world
                    .resource::<crate::world_data::WorldData>()
                    .templates
                    .get(&entry)
                else {
                    continue;
                };
                u32::from(template.faction)
            } else {
                requested
            };
            ctx.world
                .entity_mut(target)
                .insert(shared::components::UnitFactionTemplate(faction));
        }
    }

    fn change_npc_flags(ctx: &mut Ctx, targets: &[Entity], event: &SmartEvent) {
        for &target in targets {
            if let Some(mut flags) = ctx.world.get_mut::<shared::protocol::NpcFlags>(target) {
                let mask = u64::from(event.action_params[0]);
                flags.0 = match event.action_type {
                    ACTION_SET_NPC_FLAG => mask,
                    ACTION_ADD_NPC_FLAG => flags.0 | mask,
                    ACTION_REMOVE_NPC_FLAG => flags.0 & !mask,
                    _ => unreachable!(),
                };
            }
        }
    }

    fn stop_path(&mut self, ctx: &mut Ctx, failed: bool) {
        let Some(escort) = &self.escort else {
            return;
        };
        let node_id = escort.path.nodes[escort.index].id;
        ctx.world
            .entity_mut(ctx.me)
            .remove::<super::movement::PointMovement>();
        self.process_events_for(ctx, EVENT_WAYPOINT_STOPPED, None, node_id);
        self.finish_path(ctx, failed);
    }

    fn pause_path(&mut self, ctx: &mut Ctx, duration_ms: u32) {
        let Some(escort) = &mut self.escort else {
            return;
        };
        if escort.pause_ms.is_some() {
            return;
        }
        escort.pause_ms = Some(duration_ms);
        let node_id = escort.path.nodes[escort.index].id;
        ctx.world
            .entity_mut(ctx.me)
            .remove::<super::movement::PointMovement>();
        self.process_events_for(ctx, EVENT_WAYPOINT_PAUSED, None, node_id);
    }

    fn update_path(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        if self.update_node_delay(ctx, diff_ms) {
            return;
        }
        let Some(escort) = &mut self.escort else {
            return;
        };
        let Some(timer) = escort.pause_ms else {
            return;
        };
        if timer == 0 || ctx.is_engaged() {
            return;
        }
        if timer > diff_ms {
            escort.pause_ms = Some(timer - diff_ms);
            return;
        }
        escort.pause_ms = None;
        let node_id = escort.path.nodes[escort.index].id;
        let reached = escort.reached;
        self.process_events_for(ctx, EVENT_WAYPOINT_RESUMED, None, node_id);
        if reached {
            if self
                .escort
                .as_ref()
                .is_some_and(|escort| escort.node_delay_ms.is_none())
            {
                self.advance_waypoint(ctx);
            }
        } else {
            self.move_to_waypoint(ctx);
        }
    }

    fn advance_waypoint(&mut self, ctx: &mut Ctx) {
        let Some(escort) = &mut self.escort else {
            return;
        };
        if escort.index + 1 >= escort.path.nodes.len() {
            self.finish_path(ctx, false);
            return;
        }
        escort.index += 1;
        escort.reached = false;
        self.move_to_waypoint(ctx);
    }

    fn move_to_waypoint(&self, ctx: &mut Ctx) {
        let Some(escort) = &self.escort else {
            return;
        };
        let Some(node) = escort.path.nodes.get(escort.index) else {
            return;
        };
        ctx.set_walk(ctx.me, escort.walk);
        ctx.move_point(ctx.me, node.id, node.position);
    }
}

impl CreatureAi for SmartAi {
    fn store_counter(&mut self, ctx: &mut Ctx, id: u32, value: u32, reset: bool) {
        let counter = self.counters.entry(id).or_default();
        *counter = if reset {
            value
        } else {
            counter.wrapping_add(value)
        };
        self.process_events_for(ctx, EVENT_COUNTER_SET, None, id);
    }

    /// TC SmartAI::JustAppeared: RESPawn11 precedes OnReset; evade never emits11.
    fn just_appeared(&mut self, ctx: &mut Ctx) {
        if !crate::spell_cast::targets::is_alive(ctx.world, ctx.me) {
            return;
        }
        self.process_events_for(ctx, EVENT_RESPAWN, None, 0);
        self.reset(ctx);
    }

    /// `SmartScript::OnReset` (SmartScript.cpp:196-215).
    fn reset(&mut self, ctx: &mut Ctx) {
        if let Some(boss) = &mut self.boss {
            boss.reset(ctx);
        }
        for index in 0..self.events.len() {
            if self.events[index].flags & EVENT_FLAG_DONT_RESET == 0 {
                self.init_timer(index);
            }
        }
        self.process_events_for(ctx, EVENT_RESET, None, 0);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, who: Entity) {
        if let Some(boss) = &mut self.boss {
            boss.just_engaged_with(ctx);
        }
        self.process_events_for(ctx, EVENT_AGGRO, Some(who), 0);
    }

    fn enter_evade_mode(&mut self, ctx: &mut Ctx) {
        self.process_events_for(ctx, EVENT_EVADE, None, 0);
        if self.escort.is_none() {
            self.reset(ctx);
        }
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        if let Some(boss) = &mut self.boss {
            boss.just_summoned(ctx, summon);
        }
        self.process_events_for(ctx, EVENT_SUMMONED_UNIT, Some(summon), 0);
    }

    fn is_summoned_by(&mut self, ctx: &mut Ctx, summoner: Entity) {
        self.process_events_for(ctx, EVENT_JUST_SUMMONED, Some(summoner), 0);
    }

    fn summoned_creature_despawn(&mut self, ctx: &mut Ctx, summon: Entity) {
        if let Some(boss) = &mut self.boss {
            boss.summoned_creature_despawn(summon);
        }
        self.process_events_for(ctx, EVENT_SUMMON_DESPAWNED, Some(summon), 0);
    }

    fn just_died(&mut self, ctx: &mut Ctx, killer: Option<Entity>) {
        bevy::log::debug!(creature = ?ctx.me, ?killer, position = ?ctx.position(), path = ?self.escort.as_ref().map(|path| path.path_id), driver = "escort_death", "creature died");
        if let Some(boss) = &mut self.boss {
            boss.just_died(ctx);
        }
        self.finish_path(ctx, true);
        self.process_events_for(ctx, EVENT_DEATH, killer, 0);
    }

    fn movement_inform(&mut self, ctx: &mut Ctx, kind: super::MotionType, node_id: u32) {
        if kind != super::MotionType::Point {
            return;
        }
        let Some(escort) = &mut self.escort else {
            return;
        };
        if escort.path.nodes[escort.index].id != node_id {
            return;
        }
        escort.reached = true;
        let node = &escort.path.nodes[escort.index];
        escort.node_delay_ms = (node.delay_ms > 0).then_some(node.delay_ms);
        if let Some(orientation) = node.orientation {
            if let Some(mut rotation) = ctx.world.get_mut::<shared::components::Rotation>(ctx.me) {
                rotation.y = orientation + std::f32::consts::FRAC_PI_2;
            }
        }
        self.process_events_for(ctx, EVENT_WAYPOINT_REACHED, None, node_id);
        self.update_timed_list(ctx, 0);
        if self
            .escort
            .as_ref()
            .is_some_and(|escort| escort.pause_ms.is_none() && escort.node_delay_ms.is_none())
        {
            self.advance_waypoint(ctx);
        }
    }

    fn quest_accepted(&mut self, ctx: &mut Ctx, player: Entity, quest_id: u32) {
        self.process_events_for(ctx, EVENT_ACCEPTED_QUEST, Some(player), quest_id);
    }

    fn quest_rewarded(&mut self, ctx: &mut Ctx, player: Entity, quest_id: u32) {
        self.process_events_for(ctx, EVENT_REWARD_QUEST, Some(player), quest_id);
    }

    fn spell_hit(&mut self, ctx: &mut Ctx, caster: Entity, spell_id: u32) {
        self.process_events_for(ctx, EVENT_SPELLHIT, Some(caster), spell_id);
    }

    /// `SmartAI::UpdateAI` → `SmartScript::OnUpdate` → `UpdateTimer` for the timed
    /// events (SmartScript.cpp:3777-3850), after refreshing the victim.
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        ctx.update_victim();
        if self.fail_out_of_range_escort(ctx, diff_ms) {
            return;
        }
        self.update_path(ctx, diff_ms);
        self.update_follow(ctx, diff_ms);
        self.update_timed_list(ctx, diff_ms);
        for index in 0..self.events.len() {
            if self.tick_hook_cooldown(index, diff_ms) {
                continue;
            }
            let event = &self.events[index];
            if !matches!(
                event.event_type,
                EVENT_UPDATE_IC | EVENT_RANGE | EVENT_HAS_AURA
            ) || !self.in_phase(event.phase_mask)
            {
                continue;
            }
            if event.event_type == EVENT_UPDATE_IC && !ctx.is_engaged() {
                continue;
            }
            if self.states[index].timer_ms >= diff_ms {
                self.states[index].timer_ms -= diff_ms;
                continue;
            }
            // `UpdateTimer` defers a due cast while another cast is running;
            // it must not spend the repeat timer on an unattempted cast.
            if event.action_type == ACTION_CAST && ctx.is_casting() {
                self.recalc_timer(index, 1, 1);
                continue;
            }
            self.states[index].active = true;
            self.process_event(ctx, index, None, 0);
        }
    }
}
