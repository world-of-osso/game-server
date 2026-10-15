//! TC a352b1fa instance_wailing_caverns.cpp four-druid DONE notification and
//! wailing_caverns.cpp Disciple DoAction0, combat and escort events4–16. Gossip,
//! later awakening and targeted random aggro speech remain unported. Escort
//! events use their separate out-of-combat clock.
use bevy::prelude::*;
use shared::components::{CombatStatus, Health, Npc, Rotation};

use super::event_map::EventMap;

use super::instance_script::{EncounterState, InstanceScript, InstanceScripts};
use super::{CreatureAi, Ctx, Hook};
use crate::maps::InMap;

const MAP: u32 = 43;
const DISCIPLE: u32 = 3678;
const SCRIPT: &str = "npc_disciple_of_naralex";
const ACTION_ALL_DONE: i32 = 0;
const SAY_ALL_DONE: u8 = 0;
const SLEEP: u32 = 1090;
const DRUIDS_POTION: u32 = 8141;
const EVENT_SLEEP: u32 = 1;
const EVENT_DRUIDS_POTION: u32 = 2;
const POTION_HEALTH_FRACTION: f32 = 0.8;
/// DATA_LORD_COBRAHN, PYTHAS, ANACONDRA, SERPENTIS, via LoadDungeonEncounterData.
const DRUID_ENCOUNTERS: [u32; 4] = [586, 588, 585, 590];

pub(crate) fn is_creature_script(name: &str) -> bool {
    name == SCRIPT
}

pub(crate) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<InMap>(actor)?;
    if map.map_id != MAP || map.instance_id == 0 {
        return None;
    }
    Some(Box::new(Disciple::default()))
}

#[derive(Default)]
struct Disciple {
    combat_events: EventMap,
    escort_events: EventMap,
}

const PATH_FIRST_CAMP: u32 = 367800;
const PATH_SECOND_CAMP: u32 = 367801;
const PATH_NARALEX_CHAMBER: u32 = 367802;
const EVENT_FIRST_CAMP_SPEECH: u32 = 4;
const EVENT_FIRST_CAMP_SUMMONS: u32 = 5;
const EVENT_CONTINUE_SPEECH: u32 = 6;
const EVENT_CONTINUE_PATH: u32 = 7;
const FIRST_CAMP_GROUP: u32 = 0;
const SAY_FIRST_CAMP: u8 = 2;
const SAY_CONTINUE: u8 = 3;
const CONTINUE_FACING_WOW: f32 = 5.742133;
const SERPENTINE_CLEANSING: u32 = 6270;
const EVENT_SECOND_CAMP_CLEANSING: u32 = 8;
const EVENT_SECOND_CAMP_SUMMONS: u32 = 9;
const EVENT_PURIFIED_SPEECH: u32 = 10;
const EVENT_CHAMBER_PATH: u32 = 11;
const SECOND_CAMP_GROUP: u32 = 1;
const SAY_SECOND_CAMP: u8 = 4;
const SAY_PURIFIED: u8 = 5;
const EVENT_CHAMBER_SPEECH: u32 = 12;
const EVENT_APPROACH_NARALEX: u32 = 13;
const SAY_CHAMBER: u8 = 6;
const PATH_APPROACH_NARALEX: u32 = 367803;
const EVENT_RITUAL_SPEECH: u32 = 14;
const EVENT_START_AWAKENING: u32 = 15;
const EVENT_SUMMON_MOCCASINS: u32 = 16;
const SAY_RITUAL_PROTECTION: u8 = 7;
const SAY_RITUAL_BEGINS: u8 = 8;
const NARALEXS_AWAKENING: u32 = 6271;
const MOCCASIN_GROUP: u32 = 2;

impl Disciple {
    fn advance_escort(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.escort_events.update(u64::from(diff_ms));
        while let Some(event) = self.escort_events.execute_event() {
            match event {
                EVENT_FIRST_CAMP_SPEECH
                | EVENT_FIRST_CAMP_SUMMONS
                | EVENT_CONTINUE_SPEECH
                | EVENT_CONTINUE_PATH => self.run_first_camp_event(ctx, event),
                EVENT_SECOND_CAMP_CLEANSING
                | EVENT_SECOND_CAMP_SUMMONS
                | EVENT_PURIFIED_SPEECH
                | EVENT_CHAMBER_PATH => self.run_second_camp_event(ctx, event),
                EVENT_CHAMBER_SPEECH | EVENT_APPROACH_NARALEX => self.run_chamber_event(ctx, event),
                EVENT_RITUAL_SPEECH | EVENT_START_AWAKENING | EVENT_SUMMON_MOCCASINS => {
                    self.run_awakening_event(ctx, event);
                }
                _ => unreachable!("Disciple escort event {event}"),
            }
        }
    }

    fn run_awakening_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_RITUAL_SPEECH => {
                ctx.talk(SAY_RITUAL_PROTECTION);
                self.escort_events.schedule(EVENT_START_AWAKENING, 5_000);
            }
            EVENT_START_AWAKENING => {
                ctx.do_cast_self(NARALEXS_AWAKENING);
                ctx.talk(SAY_RITUAL_BEGINS);
                self.escort_events.schedule(EVENT_SUMMON_MOCCASINS, 4_000);
            }
            EVENT_SUMMON_MOCCASINS => {
                ctx.summon_creature_group(MOCCASIN_GROUP);
            }
            _ => unreachable!("Disciple awakening event {event}"),
        }
    }

    fn run_chamber_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_CHAMBER_SPEECH => {
                ctx.talk(SAY_CHAMBER);
                self.escort_events.schedule(EVENT_APPROACH_NARALEX, 5_000);
            }
            EVENT_APPROACH_NARALEX => {
                super::waypoint_path::move_path(ctx.world, ctx.me, PATH_APPROACH_NARALEX);
            }
            _ => unreachable!("Disciple chamber event {event}"),
        }
    }

    fn run_first_camp_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_FIRST_CAMP_SPEECH => {
                ctx.talk(SAY_FIRST_CAMP);
                self.escort_events.schedule(EVENT_FIRST_CAMP_SUMMONS, 7_000);
            }
            EVENT_FIRST_CAMP_SUMMONS => {
                ctx.summon_creature_group(FIRST_CAMP_GROUP);
                self.escort_events.schedule(EVENT_CONTINUE_SPEECH, 10_000);
            }
            EVENT_CONTINUE_SPEECH => {
                face_continue_path(ctx);
                ctx.talk(SAY_CONTINUE);
                self.escort_events.schedule(EVENT_CONTINUE_PATH, 3_000);
            }
            EVENT_CONTINUE_PATH => {
                super::waypoint_path::move_path(ctx.world, ctx.me, PATH_SECOND_CAMP);
            }
            _ => unreachable!("Disciple first camp event {event}"),
        }
    }

    fn run_second_camp_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_SECOND_CAMP_CLEANSING => {
                ctx.do_cast_self(SERPENTINE_CLEANSING);
                ctx.talk(SAY_SECOND_CAMP);
                self.escort_events
                    .schedule(EVENT_SECOND_CAMP_SUMMONS, 18_000);
            }
            EVENT_SECOND_CAMP_SUMMONS => {
                ctx.summon_creature_group(SECOND_CAMP_GROUP);
                self.escort_events.schedule(EVENT_PURIFIED_SPEECH, 12_000);
            }
            EVENT_PURIFIED_SPEECH => {
                ctx.talk(SAY_PURIFIED);
                self.escort_events.schedule(EVENT_CHAMBER_PATH, 5_000);
            }
            EVENT_CHAMBER_PATH => {
                super::waypoint_path::move_path(ctx.world, ctx.me, PATH_NARALEX_CHAMBER);
            }
            _ => unreachable!("Disciple second camp event {event}"),
        }
    }
}

fn face_continue_path(ctx: &mut Ctx) {
    if let Some(mut rotation) = ctx.world.get_mut::<Rotation>(ctx.me) {
        rotation.y = CONTINUE_FACING_WOW + std::f32::consts::FRAC_PI_2;
    }
    if let Some(queue) = ctx
        .world
        .get_resource::<lightyear::prelude::ReplicationDirtyQueue>()
    {
        queue.mark(ctx.me);
    }
}

impl CreatureAi for Disciple {
    fn reset(&mut self, _ctx: &mut Ctx) {
        self.combat_events.reset();
    }

    fn just_engaged_with(&mut self, _ctx: &mut Ctx, _who: Entity) {
        self.combat_events
            .schedule_range(EVENT_SLEEP, 5_000, 10_000);
        self.combat_events
            .schedule_range(EVENT_DRUIDS_POTION, 5_000, 10_000);
    }

    fn waypoint_path_ended(&mut self, _ctx: &mut Ctx, _node_id: u32, path_id: u32) {
        match path_id {
            PATH_FIRST_CAMP => self.escort_events.schedule(EVENT_FIRST_CAMP_SPEECH, 1_000),
            PATH_SECOND_CAMP => self.escort_events.schedule(EVENT_SECOND_CAMP_CLEANSING, 0),
            PATH_NARALEX_CHAMBER => self.escort_events.schedule(EVENT_CHAMBER_SPEECH, 1_000),
            PATH_APPROACH_NARALEX => self.escort_events.schedule(EVENT_RITUAL_SPEECH, 1_000),
            _ => {}
        }
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        let in_combat = ctx
            .world
            .get::<CombatStatus>(ctx.me)
            .is_some_and(|status| status.0);
        if !in_combat {
            self.advance_escort(ctx, diff_ms);
            return;
        }
        if !ctx.update_victim() {
            return;
        }
        self.combat_events.update(u64::from(diff_ms));
        while let Some(event) = self.combat_events.execute_event() {
            match event {
                EVENT_SLEEP => {
                    if let Some(target) = ctx.select_target_random(0.0, false) {
                        ctx.do_cast_at(Some(target), SLEEP, false);
                    }
                    self.combat_events.repeat_range(20_000, 30_000);
                }
                EVENT_DRUIDS_POTION => {
                    let health = ctx
                        .world
                        .get::<Health>(ctx.me)
                        .expect("living Disciple health");
                    if health.current < health.max * POTION_HEALTH_FRACTION {
                        ctx.do_cast_self(DRUIDS_POTION);
                    }
                    self.combat_events.repeat_range(10_000, 15_000);
                }
                _ => unreachable!("Disciple combat event {event}"),
            }
        }
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        // Source JustSummoned: summoned->AI()->AttackStart(me), not the player.
        crate::creature_aggro::add_threat(ctx.world, summon, ctx.me, 0.0);
    }

    fn do_action(&mut self, ctx: &mut Ctx, action: i32) {
        if action == ACTION_ALL_DONE {
            ctx.talk(SAY_ALL_DONE);
        }
    }
}

/// Called only after the base SetBossState accepted a transition, like the source override.
pub(super) fn on_boss_state_changed(
    world: &mut World,
    map: InMap,
    encounter: u32,
    state: EncounterState,
) {
    let druid_finished = state == EncounterState::Done && DRUID_ENCOUNTERS.contains(&encounter);
    if map.map_id != MAP || !druid_finished {
        return;
    }
    let ready = world
        .get_resource::<InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&map))
        .is_some_and(all_druids_done);
    if !ready {
        return;
    }
    if let Some(disciple) = find_disciple(world, map) {
        super::dispatch(world, disciple, Hook::DoAction(ACTION_ALL_DONE));
    }
}

fn all_druids_done(script: &InstanceScript) -> bool {
    DRUID_ENCOUNTERS.iter().all(|encounter| {
        script
            .bosses
            .iter()
            .any(|boss| boss.encounter_id == *encounter && boss.state == EncounterState::Done)
    })
}

/// Source GetCreature(DATA_DISCIPLE): only the object in this instance copy.
fn find_disciple(world: &mut World, map: InMap) -> Option<Entity> {
    world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find_map(|(actor, npc, copy)| {
            (npc.template_id == DISCIPLE && *copy == map).then_some(actor)
        })
}
