//! Grim Batol (map 670), ported from TrinityCore
//! src/server/scripts/EasternKingdoms/GrimBatol/ (a352b1fa): `grim_batol.h` and
//! `instance_grim_batol.cpp` (`SetBossNumber(4)`, `LoadObjectData(creatureData)`,
//! `LoadDungeonEncounterData(encounters)`).
//!
//! Deviation: TrinityCore has no creature scripts for Grim Batol's bosses, so none of
//! them ever sets its boss state and no kill there locks anyone. Here each boss the
//! instance script's `creatureData` names runs a spell-less `BossAI`
//! (data-selected SmartAI lifecycle): engaging sets its boss `IN_PROGRESS`, a wipe `NOT_STARTED`,
//! its death `DONE`, as `BossAI::_JustEngagedWith` / `_Reset` / `_JustDied` do.

use super::instance_script::{BossInfo, EncounterState, InstanceScript};

/// `GBTScriptName`.
pub const INSTANCE_GRIM_BATOL: &str = "instance_grim_batol";

/// `GBDataTypes` boss ids.
pub const BOSS_GENERAL_UMBRISS: u32 = 0;
pub const BOSS_FORGEMASTER_THRONGUS: u32 = 1;
pub const BOSS_DRAHGA_SHADOWBURNER: u32 = 2;
pub const BOSS_ERUDAX: u32 = 3;

/// `creatureData` (`ObjectData`): creature entry → boss id.
pub const BOSS_CREATURES: [(u32, u32); 4] = [
    (39625, BOSS_GENERAL_UMBRISS),
    (40177, BOSS_FORGEMASTER_THRONGUS),
    (40319, BOSS_DRAHGA_SHADOWBURNER),
    (40484, BOSS_ERUDAX),
];

/// `encounters` (`DungeonEncounterData`), by boss id.
const ENCOUNTERS: [u32; 4] = [1051, 1050, 1048, 1049];

/// `instance_grim_batol_InstanceMapScript`.
pub fn instance_script(name: &str) -> Option<InstanceScript> {
    (name == INSTANCE_GRIM_BATOL).then(|| InstanceScript {
        name: INSTANCE_GRIM_BATOL,
        bosses: ENCOUNTERS
            .iter()
            .map(|&encounter_id| BossInfo {
                state: EncounterState::NotStarted,
                encounter_id,
            })
            .collect(),
        boss_creatures: BOSS_CREATURES.to_vec(),
    })
}
