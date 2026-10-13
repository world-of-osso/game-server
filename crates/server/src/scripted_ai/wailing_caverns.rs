//! TC a352b1fa instance_wailing_caverns.cpp four-druid DONE notification and
//! wailing_caverns.cpp Disciple DoAction0. Escort/gossip/combat are not ported here.
use bevy::prelude::*;
use shared::components::Npc;

use super::instance_script::{EncounterState, InstanceScript, InstanceScripts};
use super::{CreatureAi, Ctx, Hook};
use crate::maps::InMap;

const MAP: u32 = 43;
const DISCIPLE: u32 = 3678;
const SCRIPT: &str = "npc_disciple_of_naralex";
const ACTION_ALL_DONE: i32 = 0;
const SAY_ALL_DONE: u8 = 0;
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
    Some(Box::new(Disciple))
}

struct Disciple;

impl CreatureAi for Disciple {
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
