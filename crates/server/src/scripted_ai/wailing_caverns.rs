//! The bounded Lord Cobrahn encounter, identified by local data, not a C++ port.
//! `tdb_smart_scripts` entry3669/event6/action34 sets instance data0 to DONE3;
//! `dungeon_encounter` names Lord Cobrahn as586 on map43 at Normal difficulty.
//! Abilities and dialogue remain entirely in SmartAI rows and creature_text.

use super::instance_script::{BossInfo, EncounterState, InstanceScript};

const INSTANCE_SCRIPT: &str = "instance_wailing_caverns";
const LORD_COBRAHN: u32 = 3669;
const COBRAHN_DATA: u32 = 0;
const COBRAHN_ENCOUNTER: u32 = 586;

pub fn instance_script(name: &str) -> Option<InstanceScript> {
    (name == INSTANCE_SCRIPT).then(|| InstanceScript {
        name: INSTANCE_SCRIPT,
        bosses: vec![BossInfo {
            state: EncounterState::NotStarted,
            encounter_id: COBRAHN_ENCOUNTER,
        }],
        boss_creatures: vec![(LORD_COBRAHN, COBRAHN_DATA)],
    })
}
