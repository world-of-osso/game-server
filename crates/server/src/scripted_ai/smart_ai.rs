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
//! Modelled: events RESET, HAS_AURA, SPELLHIT, LINK; actions CAST, SET_EVENT_PHASE,
//! CALL_KILLEDMONSTER, DIE; targets NONE, SELF, ACTION_INVOKER. Other event types never
//! run and other actions or targets do nothing (counted when loaded).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use rand::Rng;
use rusqlite::{Connection, OpenFlags};

use super::{CreatureAi, Ctx};

// `SMART_EVENT` (SmartScriptMgr.h).
const EVENT_UPDATE_IC: u32 = 0;
const EVENT_SPELLHIT: u32 = 8;
const EVENT_HAS_AURA: u32 = 23;
const EVENT_RESET: u32 = 25;
const EVENT_LINK: u32 = 61;
// `SMART_ACTION`.
const ACTION_CAST: u32 = 11;
const ACTION_SET_EVENT_PHASE: u32 = 22;
const ACTION_CALL_KILLEDMONSTER: u32 = 33;
const ACTION_DIE: u32 = 37;
// `SMART_TARGET`.
const TARGET_NONE: u32 = 0;
const TARGET_SELF: u32 = 1;
const TARGET_VICTIM: u32 = 2;
const TARGET_ACTION_INVOKER: u32 = 7;
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
}

/// Creature entry -> its SmartAI rows, by id.
#[derive(Resource, Debug, Default)]
pub(crate) struct SmartScripts(pub HashMap<u32, Arc<Vec<SmartEvent>>>);

const SMART_QUERY: &str = "SELECT s.entryorguid, s.id, s.link, s.event_type, s.event_phase_mask, \
     s.event_chance, s.event_flags, s.event_param1, s.event_param2, s.event_param3, s.event_param4, \
     s.event_param5, s.action_type, s.action_param1, s.action_param2, s.action_param3, \
     s.action_param4, s.action_param5, s.action_param6, s.action_param7, s.target_type \
     FROM tdb_smart_scripts s \
     JOIN content_creature_template t ON t.entry = s.entryorguid AND t.AIName = 'SmartAI' \
     WHERE s.source_type = 0 AND s.entryorguid > 0 ORDER BY s.entryorguid, s.id";

pub(crate) fn load_smart_scripts(path: &Path) -> rusqlite::Result<SmartScripts> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(SMART_QUERY)?;
    let rows = statement.query_map([], |row| {
        let int = |index: usize| row.get::<_, u32>(index);
        Ok((
            int(0)?,
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
            },
        ))
    })?;
    let mut scripts: HashMap<u32, Vec<SmartEvent>> = HashMap::new();
    for row in rows {
        let (entry, event) = row?;
        scripts.entry(entry).or_default().push(event);
    }
    let rows: Vec<&SmartEvent> = scripts.values().flatten().collect();
    let unmodelled = rows
        .iter()
        .filter(|event| {
            !matches!(
                event.event_type,
                EVENT_UPDATE_IC | EVENT_SPELLHIT | EVENT_HAS_AURA | EVENT_RESET | EVENT_LINK
            ) || !matches!(
                event.action_type,
                ACTION_CAST | ACTION_SET_EVENT_PHASE | ACTION_CALL_KILLEDMONSTER | ACTION_DIE
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
}

/// The SmartAI of a creature of `entry`, if it has rows.
pub(crate) fn creature_ai(world: &World, entry: u32) -> Option<Box<dyn CreatureAi>> {
    let events = world.get_resource::<SmartScripts>()?.0.get(&entry)?.clone();
    let states = vec![EventState::default(); events.len()];
    Some(Box::new(SmartAi {
        events,
        states,
        phase: 0,
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
            EVENT_LINK | EVENT_RESET => self.process_action(ctx, index, invoker),
            EVENT_UPDATE_IC => {
                if ctx.is_engaged() {
                    self.recalc_timer(index, event.event_params[2], event.event_params[3]);
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
        let targets = match event.target_type {
            TARGET_SELF => vec![ctx.me],
            TARGET_VICTIM => ctx.victim().into_iter().collect(),
            TARGET_ACTION_INVOKER => invoker.into_iter().collect(),
            _ => Vec::new(),
        };
        match event.action_type {
            ACTION_CAST => {
                if !Self::cast_targets(ctx, &event, targets) {
                    // `SmartScript::RetryLater`: retry next tick, retaining the chance roll.
                    self.recalc_timer(index, 1, 1);
                    self.states[index].run_once = false;
                    self.states[index].ignore_chance_roll = true;
                    return;
                }
            }
            ACTION_SET_EVENT_PHASE => self.phase = event.action_params[0],
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
            _ => {}
        }
        if event.link != 0 && event.link != event.id {
            if let Some(linked) = self.events.iter().position(|e| e.id == event.link) {
                self.process_event(ctx, linked, invoker, 0);
            }
        }
    }
}

impl CreatureAi for SmartAi {
    /// `SmartScript::OnReset` (SmartScript.cpp:196-215).
    fn reset(&mut self, ctx: &mut Ctx) {
        ctx.world
            .entity_mut(ctx.me)
            .remove::<CombatMovementDisabled>();
        for index in 0..self.events.len() {
            if self.events[index].flags & EVENT_FLAG_DONT_RESET == 0 {
                self.init_timer(index);
                self.states[index].run_once = false;
            }
        }
        self.process_events_for(ctx, EVENT_RESET, None, 0);
    }

    fn spell_hit(&mut self, ctx: &mut Ctx, caster: Entity, spell_id: u32) {
        self.process_events_for(ctx, EVENT_SPELLHIT, Some(caster), spell_id);
    }

    /// `SmartAI::UpdateAI` → `SmartScript::OnUpdate` → `UpdateTimer` for the timed
    /// events (SmartScript.cpp:3777-3850), after refreshing the victim.
    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        ctx.update_victim();
        for index in 0..self.events.len() {
            let event = &self.events[index];
            if !matches!(event.event_type, EVENT_UPDATE_IC | EVENT_HAS_AURA)
                || !self.in_phase(event.phase_mask)
                || (event.event_type == EVENT_UPDATE_IC && !ctx.is_engaged())
            {
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
