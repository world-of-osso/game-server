//! TC a352b1fa instance_wailing_caverns.cpp four-druid DONE notification and
//! wailing_caverns.cpp Disciple DoAction0 and combat spells. Escort, gossip and
//! targeted random aggro speech remain unported.
use bevy::prelude::*;
use shared::components::{Health, Npc};

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

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
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
