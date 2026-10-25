//! TC a352b1fa instance_wailing_caverns.cpp four-druid DONE notification and
//! wailing_caverns.cpp Disciple DoAction0, gossip, combat and escort events1–27.
//! Natural gossip eligibility remains source-data gated. Targeted aggro Say
//! preserves the nearby audience. Escort
//! events use their separate out-of-combat clock.
use bevy::prelude::*;
use shared::components::{
    CombatStatus, Health, Npc, Rotation, StandState, UnitFactionTemplate, UnitPose,
};

use shared::protocol::NpcFlags;

use super::event_map::EventMap;

use super::instance_script::{EncounterState, InstanceScript, InstanceScripts};
use super::{CreatureAi, Ctx, Hook};
use crate::maps::InMap;

const MAP: u32 = 43;
const DISCIPLE: u32 = 3678;
const SCRIPT: &str = "npc_disciple_of_naralex";
const ACTION_ALL_DONE: i32 = 0;
const SAY_ALL_DONE: u8 = 0;
const SAY_AGGRO: u8 = 10;
const AGGRO_SPEECH_CHANCE: u32 = 30;
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
const MARK_OF_THE_WILD: u32 = 5232;
const GOSSIP_MENU_EVENT: u32 = 202;
const EVENT_START: u32 = 1;
const EVENT_START_SPEECH: u32 = 2;
const EVENT_START_PATH: u32 = 3;
const SAY_START: u8 = 1;
const FACTION_ESCORTEE: u32 = 250;
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
const EVENT_NARALEX_RESTLESS: u32 = 17;
const EVENT_SUMMON_ECTOPLASMS: u32 = 18;
const EVENT_NARALEX_AGONY: u32 = 19;
const EVENT_SUMMON_MUTANUS: u32 = 20;
const ECTOPLASM_GROUP: u32 = 3;
const MUTANUS_GROUP: u32 = 4;
const NARALEX: u32 = 3679;
const EMOTE_NARALEX_RESTLESS: u8 = 0;
const EMOTE_NARALEX_AGONY: u8 = 1;
const EMOTE_NARALEX_VISION: u8 = 2;
const EVENT_WAKE_NARALEX: u32 = 21;
const EVENT_END_AWAKENING: u32 = 22;
const EVENT_NARALEX_THANKS: u32 = 23;
const EVENT_NARALEX_FAREWELL: u32 = 24;
const EVENT_OWL_FORM: u32 = 25;
const EVENT_FLIGHT: u32 = 26;
const EVENT_DESPAWN: u32 = 27;
const SAY_NARALEX_AWAKE: u8 = 3;
const SAY_NARALEX_THANKS: u8 = 4;
const SAY_NARALEX_FAREWELL: u8 = 5;
const SAY_AWAKENED: u8 = 9;
const OWL_FORM: u32 = 8153;
const FACTION_FRIENDLY: u32 = 35;
const PATH_DISCIPLE_EXIT: u32 = 367804;
const PATH_NARALEX_EXIT: u32 = 367900;

impl Disciple {
    fn advance_escort(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        self.escort_events.update(u64::from(diff_ms));
        while let Some(event) = self.escort_events.execute_event() {
            match event {
                EVENT_START | EVENT_START_SPEECH | EVENT_START_PATH => {
                    self.run_start_event(ctx, event)
                }
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
                EVENT_NARALEX_RESTLESS
                | EVENT_SUMMON_ECTOPLASMS
                | EVENT_NARALEX_AGONY
                | EVENT_SUMMON_MUTANUS => self.run_later_ritual_event(ctx, event),
                EVENT_WAKE_NARALEX
                | EVENT_END_AWAKENING
                | EVENT_NARALEX_THANKS
                | EVENT_NARALEX_FAREWELL => self.run_wake_up_event(ctx, event),
                EVENT_OWL_FORM | EVENT_FLIGHT | EVENT_DESPAWN => self.run_exit_event(ctx, event),
                _ => unreachable!("Disciple escort event {event}"),
            }
        }
    }

    fn run_start_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_START => {
                ctx.world
                    .entity_mut(ctx.me)
                    .insert(UnitFactionTemplate(FACTION_ESCORTEE));
                ctx.world
                    .get_mut::<NpcFlags>(ctx.me)
                    .expect("Disciple NPC flags")
                    .0 &= !NpcFlags::GOSSIP;
                mark_replication_dirty(ctx.world, ctx.me);
                self.escort_events.schedule(EVENT_START_SPEECH, 10_000);
            }
            EVENT_START_SPEECH => {
                ctx.talk(SAY_START);
                self.escort_events.schedule(EVENT_START_PATH, 3_500);
            }
            EVENT_START_PATH => {
                super::waypoint_path::move_path(ctx.world, ctx.me, PATH_FIRST_CAMP);
            }
            _ => unreachable!("Disciple start event {event}"),
        }
    }

    fn run_later_ritual_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_NARALEX_RESTLESS => {
                talk_naralex(ctx, EMOTE_NARALEX_RESTLESS);
                self.escort_events.schedule(EVENT_SUMMON_ECTOPLASMS, 25_000);
            }
            EVENT_SUMMON_ECTOPLASMS => {
                ctx.summon_creature_group(ECTOPLASM_GROUP);
                self.escort_events.schedule(EVENT_NARALEX_AGONY, 20_000);
            }
            EVENT_NARALEX_AGONY => {
                talk_naralex(ctx, EMOTE_NARALEX_AGONY);
                self.escort_events.schedule(EVENT_SUMMON_MUTANUS, 30_000);
            }
            EVENT_SUMMON_MUTANUS => {
                talk_naralex(ctx, EMOTE_NARALEX_VISION);
                ctx.summon_creature_group(MUTANUS_GROUP);
                self.escort_events.schedule(EVENT_WAKE_NARALEX, 25_000);
            }
            _ => unreachable!("Disciple later ritual event {event}"),
        }
    }

    fn run_wake_up_event(&mut self, ctx: &mut Ctx, event: u32) {
        match event {
            EVENT_WAKE_NARALEX => {
                wake_naralex(ctx);
                self.escort_events.schedule(EVENT_END_AWAKENING, 3_000);
            }
            EVENT_END_AWAKENING => {
                crate::auras::remove_auras(
                    ctx.world,
                    ctx.me,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == NARALEXS_AWAKENING,
                );
                ctx.talk(SAY_AWAKENED);
                self.escort_events.schedule(EVENT_NARALEX_THANKS, 3_000);
            }
            EVENT_NARALEX_THANKS => {
                talk_naralex(ctx, SAY_NARALEX_THANKS);
                self.escort_events.schedule(EVENT_NARALEX_FAREWELL, 6_000);
            }
            EVENT_NARALEX_FAREWELL => {
                talk_naralex(ctx, SAY_NARALEX_FAREWELL);
                self.escort_events.schedule(EVENT_OWL_FORM, 3_000);
            }
            _ => unreachable!("Disciple wake-up event {event}"),
        }
    }

    fn run_exit_event(&mut self, ctx: &mut Ctx, event: u32) {
        let naralex = find_naralex(ctx);
        match event {
            EVENT_OWL_FORM => {
                cast_departure_forms(ctx, naralex);
                self.escort_events.schedule(EVENT_FLIGHT, 8_000);
            }
            EVENT_FLIGHT => {
                if let Some(naralex) = naralex {
                    start_exit_flight(ctx.world, naralex, PATH_NARALEX_EXIT);
                }
                start_exit_flight(ctx.world, ctx.me, PATH_DISCIPLE_EXIT);
            }
            EVENT_DESPAWN => {
                if let Some(naralex) = naralex {
                    super::smart_ai::lifetimes::despawn_creature(ctx.world, naralex, 0);
                }
                super::smart_ai::lifetimes::despawn_creature(ctx.world, ctx.me, 0);
            }
            _ => unreachable!("Disciple exit event {event}"),
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
                self.escort_events.schedule(EVENT_NARALEX_RESTLESS, 15_000);
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

fn find_naralex(ctx: &mut Ctx) -> Option<Entity> {
    let map = *ctx
        .world
        .get::<InMap>(ctx.me)
        .expect("Disciple instance map");
    find_instance_creature(ctx.world, map, NARALEX)
}

fn talk_naralex(ctx: &mut Ctx, group: u8) {
    if let Some(naralex) = find_naralex(ctx) {
        super::creature_text::talk(ctx.world, naralex, group);
    }
}

fn wake_naralex(ctx: &mut Ctx) {
    if let Some(naralex) = find_naralex(ctx) {
        ctx.world
            .get_mut::<UnitPose>(naralex)
            .expect("Naralex unit pose")
            .stand_state = StandState::Stand;
        mark_replication_dirty(ctx.world, naralex);
        super::creature_text::talk(ctx.world, naralex, SAY_NARALEX_AWAKE);
    }
}

fn cast_departure_forms(ctx: &mut Ctx, naralex: Option<Entity>) {
    ctx.do_cast_self(OWL_FORM);
    if let Some(naralex) = naralex {
        Ctx {
            world: ctx.world,
            me: naralex,
        }
        .do_cast_self(OWL_FORM);
    }
    ctx.world
        .entity_mut(ctx.me)
        .insert(UnitFactionTemplate(FACTION_FRIENDLY));
    mark_replication_dirty(ctx.world, ctx.me);
}

fn start_exit_flight(world: &mut World, actor: Entity, path: u32) {
    world
        .entity_mut(actor)
        .insert(super::waypoint_path::GravityDisabled);
    super::waypoint_path::move_path(world, actor, path);
}

fn mark_replication_dirty(world: &World, actor: Entity) {
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        queue.mark(actor);
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
    fn gossip_hello(&mut self, ctx: &mut Ctx, player: Entity) -> bool {
        ctx.do_cast_at(Some(player), MARK_OF_THE_WILD, false);
        let map = *ctx.world.get::<InMap>(ctx.me).expect("Disciple map");
        let ready = ctx
            .world
            .resource::<InstanceScripts>()
            .0
            .get(&map)
            .is_some_and(all_druids_done);
        if ready {
            crate::npc_interaction::prepare_gossip_menu(
                ctx.world,
                player,
                ctx.me,
                GOSSIP_MENU_EVENT,
            );
        }
        ready
    }

    fn gossip_select(&mut self, ctx: &mut Ctx, player: Entity) -> bool {
        crate::npc_interaction::close_gossip_menu(ctx.world, player, ctx.me);
        self.escort_events.schedule(EVENT_START, 0);
        false
    }

    fn reset(&mut self, _ctx: &mut Ctx) {
        self.combat_events.reset();
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, who: Entity) {
        self.combat_events
            .schedule_range(EVENT_SLEEP, 5_000, 10_000);
        self.combat_events
            .schedule_range(EVENT_DRUIDS_POTION, 5_000, 10_000);
        let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..100);
        if roll < AGGRO_SPEECH_CHANCE {
            super::creature_text::talk_with_target(ctx.world, ctx.me, SAY_AGGRO, Some(who));
        }
    }

    fn waypoint_path_ended(&mut self, _ctx: &mut Ctx, _node_id: u32, path_id: u32) {
        match path_id {
            PATH_FIRST_CAMP => self.escort_events.schedule(EVENT_FIRST_CAMP_SPEECH, 1_000),
            PATH_SECOND_CAMP => self.escort_events.schedule(EVENT_SECOND_CAMP_CLEANSING, 0),
            PATH_NARALEX_CHAMBER => self.escort_events.schedule(EVENT_CHAMBER_SPEECH, 1_000),
            PATH_APPROACH_NARALEX => self.escort_events.schedule(EVENT_RITUAL_SPEECH, 1_000),
            PATH_DISCIPLE_EXIT => self.escort_events.schedule(EVENT_DESPAWN, 0),
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
    if let Some(disciple) = find_instance_creature(world, map, DISCIPLE) {
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

/// Source instance GetCreature: only the requested entry in this copy.
fn find_instance_creature(world: &mut World, map: InMap, entry: u32) -> Option<Entity> {
    world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find_map(|(actor, npc, copy)| (npc.template_id == entry && *copy == map).then_some(actor))
}
