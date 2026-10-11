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
//! Modelled: events UPDATE_IC, AGGRO, RESET, HAS_AURA, SPELLHIT, LINK; actions TALK,
//! CAST, SET_EVENT_PHASE,
//! CALL_KILLEDMONSTER, DIE; targets NONE, SELF, ACTION_INVOKER. Other event types never
//! run and other actions or targets do nothing (counted when loaded).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use rand::Rng;
use rusqlite::{Connection, OpenFlags};

use super::{CreatureAi, Ctx};

#[path = "smart_paths.rs"]
mod paths;

#[cfg(test)]
#[path = "smart_escort_tests.rs"]
mod escort_tests;

// `SMART_EVENT` (SmartScriptMgr.h).
const EVENT_UPDATE_IC: u32 = 0;
const EVENT_AGGRO: u32 = 4;
const EVENT_DEATH: u32 = 6;
const EVENT_SPELLHIT: u32 = 8;
const EVENT_HAS_AURA: u32 = 23;
const EVENT_RESET: u32 = 25;
const EVENT_LINK: u32 = 61;
const EVENT_ACCEPTED_QUEST: u32 = 19;
const EVENT_REWARD_QUEST: u32 = 20;
const EVENT_WAYPOINT_REACHED: u32 = 40;
const EVENT_WAYPOINT_PAUSED: u32 = 55;
const EVENT_WAYPOINT_RESUMED: u32 = 56;
const EVENT_WAYPOINT_STOPPED: u32 = 57;
const EVENT_WAYPOINT_ENDED: u32 = 58;
// `SMART_ACTION`.
const ACTION_TALK: u32 = 1;
const ACTION_FAIL_QUEST: u32 = 6;
const ACTION_COMPLETE_QUEST: u32 = 152;
const ACTION_CAST: u32 = 11;
const ACTION_SET_EVENT_PHASE: u32 = 22;
const ACTION_CALL_KILLEDMONSTER: u32 = 33;
const ACTION_DIE: u32 = 37;
const ACTION_WP_START: u32 = 53;
const ACTION_WP_PAUSE: u32 = 54;
const ACTION_WP_RESUME: u32 = 65;
const ACTION_STORE_TARGET_LIST: u32 = 64;
const ACTION_CALL_TIMED_ACTION_LIST: u32 = 80;
const ACTION_REMOVE_NPC_FLAG: u32 = 83;
// `SMART_TARGET`.
const TARGET_NONE: u32 = 0;
const TARGET_SELF: u32 = 1;
const TARGET_VICTIM: u32 = 2;
const TARGET_ACTION_INVOKER: u32 = 7;
const TARGET_STORED: u32 = 12;
const TARGET_INVOKER_PARTY: u32 = 16;
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
pub(crate) struct SmartScripts(
    pub HashMap<u32, Arc<Vec<SmartEvent>>>,
    HashMap<u32, Arc<Vec<SmartEvent>>>,
    HashMap<u32, Arc<paths::WaypointPath>>,
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
    let rows: Vec<&SmartEvent> = scripts.values().flatten().collect();
    let unmodelled = rows
        .iter()
        .filter(|event| {
            !matches!(
                event.event_type,
                EVENT_UPDATE_IC
                    | EVENT_AGGRO
                    | EVENT_SPELLHIT
                    | EVENT_HAS_AURA
                    | EVENT_RESET
                    | EVENT_LINK
            ) || !matches!(
                event.action_type,
                ACTION_TALK
                    | ACTION_CAST
                    | ACTION_SET_EVENT_PHASE
                    | ACTION_CALL_KILLEDMONSTER
                    | ACTION_DIE
            )
        })
        .count();
    info!(
        "Loaded SmartAI scripts of {} creatures ({} of {} rows use unmodelled events or actions)",
        scripts.len(),
        unmodelled,
        rows.len()
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
    ))
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
    timed_list: Option<TimedList>,
    escort: Option<paths::EscortPath>,
}

struct TimedList {
    events: Arc<Vec<SmartEvent>>,
    index: usize,
    timer_ms: u32,
    invoker: Option<Entity>,
}

/// The SmartAI of a creature of `entry`, if it has rows.
pub(crate) fn creature_ai(world: &World, creature: Entity) -> Option<Box<dyn CreatureAi>> {
    let entry = world.get::<shared::components::Npc>(creature)?.template_id;
    let events = world.get_resource::<SmartScripts>()?.0.get(&entry)?.clone();
    let states = vec![EventState::default(); events.len()];
    Some(Box::new(SmartAi {
        events,
        states,
        phase: 0,
        boss: super::wailing_caverns::boss_ai(world, creature),
        stored_targets: HashMap::new(),
        timed_list: None,
        escort: None,
    }))
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
            EVENT_LINK | EVENT_RESET | EVENT_AGGRO | EVENT_DEATH => {
                self.process_action(ctx, index, invoker)
            }
            EVENT_UPDATE_IC => {
                if ctx.is_engaged() {
                    self.recalc_timer(index, event.event_params[2], event.event_params[3]);
                    self.process_action(ctx, index, invoker);
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
                let path_id = self.escort.as_ref().map_or(0, |escort| escort.path_id);
                if (event.event_params[0] == 0 || event.event_params[0] == spell)
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
            TARGET_ACTION_INVOKER | TARGET_INVOKER_PARTY => invoker.into_iter().collect(),
            TARGET_STORED => self
                .stored_targets
                .get(&event.target_params[0])
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        match event.action_type {
            ACTION_TALK => ctx.talk(event.action_params[0] as u8),
            ACTION_CAST => {
                return Self::cast_targets(ctx, event, targets);
            }
            ACTION_SET_EVENT_PHASE => self.phase = event.action_params[0],
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
            ACTION_DIE => {
                if ctx.is_alive() {
                    ctx.kill_self();
                }
            }
            ACTION_STORE_TARGET_LIST => {
                self.stored_targets.insert(event.action_params[0], targets);
            }
            ACTION_CALL_TIMED_ACTION_LIST => {
                self.start_timed_list(ctx, event.action_params[0], invoker)
            }
            ACTION_REMOVE_NPC_FLAG => {
                for target in targets {
                    if let Some(mut flags) = ctx.world.get_mut::<shared::protocol::NpcFlags>(target)
                    {
                        flags.0 &= !u64::from(event.action_params[0]);
                    }
                }
            }
            ACTION_WP_START => self.start_path(ctx, event, invoker),
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

    fn start_timed_list(&mut self, ctx: &Ctx, id: u32, invoker: Option<Entity>) {
        let events = ctx.world.resource::<SmartScripts>().1.get(&id).cloned();
        let Some(events) = events else {
            warn!(id, "SmartAI timed action list is absent");
            return;
        };
        let timer_ms = events.first().map_or(0, |row| row.event_params[0]);
        self.timed_list = Some(TimedList {
            events,
            index: 0,
            timer_ms,
            invoker,
        });
    }

    fn update_timed_list(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let Some(mut list) = self.timed_list.take() else {
            return;
        };
        if list.timer_ms > diff_ms {
            list.timer_ms -= diff_ms;
            self.timed_list = Some(list);
            return;
        }
        while let Some(event) = list.events.get(list.index) {
            if !self.execute_action(ctx, event, list.invoker) {
                list.timer_ms = 1;
                self.timed_list = Some(list);
                return;
            }
            list.index += 1;
            let Some(next) = list.events.get(list.index) else {
                return;
            };
            list.timer_ms = next.event_params[0];
            if list.timer_ms > 0 {
                self.timed_list = Some(list);
                return;
            }
        }
    }

    fn start_path(&mut self, ctx: &mut Ctx, event: &SmartEvent, invoker: Option<Entity>) {
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
            path,
            index: 0,
            pause_ms: None,
            reached: false,
        });
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
            self.advance_waypoint(ctx);
        } else {
            self.move_to_waypoint(ctx);
        }
    }

    fn advance_waypoint(&mut self, ctx: &mut Ctx) {
        let Some(escort) = &mut self.escort else {
            return;
        };
        escort.index += 1;
        escort.reached = false;
        if escort.index >= escort.path.nodes.len() {
            let last = escort.path.nodes.last().map_or(0, |node| node.id);
            self.process_events_for(ctx, EVENT_WAYPOINT_ENDED, None, last);
            self.escort = None;
            return;
        }
        self.move_to_waypoint(ctx);
    }

    fn move_to_waypoint(&self, ctx: &mut Ctx) {
        let Some(escort) = &self.escort else {
            return;
        };
        let Some(node) = escort.path.nodes.get(escort.index) else {
            return;
        };
        ctx.set_walk(ctx.me, escort.path.walk);
        ctx.move_point(ctx.me, node.id, node.position);
    }
}

impl CreatureAi for SmartAi {
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

    fn just_died(&mut self, ctx: &mut Ctx, killer: Option<Entity>) {
        if let Some(boss) = &mut self.boss {
            boss.just_died(ctx);
        }
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
        self.process_events_for(ctx, EVENT_WAYPOINT_REACHED, None, node_id);
        self.update_timed_list(ctx, 0);
        if self
            .escort
            .as_ref()
            .is_some_and(|escort| escort.pause_ms.is_none())
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
        self.update_path(ctx, diff_ms);
        self.update_timed_list(ctx, diff_ms);
        for index in 0..self.events.len() {
            let event = &self.events[index];
            if !matches!(event.event_type, EVENT_UPDATE_IC | EVENT_HAS_AURA)
                || !self.in_phase(event.phase_mask)
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
