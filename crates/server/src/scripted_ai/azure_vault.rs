//! Local TDB instance_azure_vault and DB2 encounter identities.
//! Uses the existing spell-less BossAI lifecycle, not invented Azure combat mechanics.
use super::CreatureAi;
use super::instance_script::{BossInfo, EncounterState, InstanceScript};
use bevy::prelude::*;
const NAME: &str = "instance_azure_vault";
const BOSSES: [(u32, u32); 4] = [(186644, 0), (186739, 1), (186737, 2), (186738, 3)];
const ENCOUNTERS: [u32; 4] = [2582, 2585, 2583, 2584];
pub fn boss_id(entry: u32) -> Option<u32> {
    BOSSES
        .iter()
        .find_map(|&(id, boss)| (entry == id).then_some(boss))
}
pub fn instance_script(name: &str) -> Option<InstanceScript> {
    (name == NAME).then(|| InstanceScript {
        name: NAME,
        bosses: ENCOUNTERS
            .iter()
            .map(|&encounter_id| BossInfo {
                state: EncounterState::NotStarted,
                encounter_id,
            })
            .collect(),
        boss_creatures: &BOSSES,
    })
}
pub fn creature_ai(world: &World, creature: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(creature)?;
    let script = world
        .get_resource::<crate::maps::MapStore>()?
        .instance_scripts
        .get(&map.map_id)?;
    if script != NAME {
        return None;
    }
    let entry = world.get::<shared::components::Npc>(creature)?.template_id;
    Some(Box::new(super::grim_batol::InstanceBossAi::new(boss_id(
        entry,
    )?)))
}
