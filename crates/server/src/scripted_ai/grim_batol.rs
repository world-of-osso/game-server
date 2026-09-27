//! Grim Batol (map 670), ported from TrinityCore
//! src/server/scripts/EasternKingdoms/GrimBatol/ (a352b1fa): `grim_batol.h` and
//! `instance_grim_batol.cpp` (`SetBossNumber(4)`, `LoadObjectData(creatureData)`,
//! `LoadDungeonEncounterData(encounters)`).
//!
//! Deviation: TrinityCore has no creature scripts for Grim Batol's bosses, so none of
//! them ever sets its boss state and no kill there locks anyone. Here each boss the
//! instance script's `creatureData` names runs a spell-less `BossAI`
//! ([`InstanceBossAi`]): engaging sets its boss `IN_PROGRESS`, a wipe `NOT_STARTED`,
//! its death `DONE`, as `BossAI::_JustEngagedWith` / `_Reset` / `_JustDied` do.

use bevy::prelude::*;

use super::boss_ai::{BossAi, BossScript, boss_update_ai};
use super::instance_script::{BossInfo, EncounterState, InstanceScript};
use super::{CreatureAi, Ctx};

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
        boss_creatures: &BOSS_CREATURES,
    })
}

/// The boss id of a Grim Batol boss creature entry.
pub fn boss_id_of(entry: u32) -> Option<u32> {
    BOSS_CREATURES
        .iter()
        .find_map(|&(boss_entry, boss_id)| (boss_entry == entry).then_some(boss_id))
}

/// The AI of a boss `creatureData` names, in a map whose instance script is
/// `instance_grim_batol`.
pub fn creature_ai(world: &World, creature: Entity) -> Option<Box<dyn CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(creature)?;
    let script = world
        .get_resource::<crate::maps::MapStore>()
        .and_then(|store| store.instance_scripts.get(&map.map_id))?;
    if script != INSTANCE_GRIM_BATOL {
        return None;
    }
    let entry = world.get::<shared::components::Npc>(creature)?.template_id;
    let boss_id = boss_id_of(entry)?;
    Some(Box::new(InstanceBossAi {
        boss: BossAi::new(boss_id),
    }))
}

/// `BossAI` without a script's events: the boss state follows engage, wipe and death.
pub struct InstanceBossAi {
    boss: BossAi,
}

impl BossScript for InstanceBossAi {
    fn boss_ai(&mut self) -> &mut BossAi {
        &mut self.boss
    }

    fn execute_event(&mut self, _ctx: &mut Ctx, _event_id: u32) {}
}

impl CreatureAi for InstanceBossAi {
    fn reset(&mut self, ctx: &mut Ctx) {
        self.boss.reset(ctx);
    }

    fn just_engaged_with(&mut self, ctx: &mut Ctx, _who: Entity) {
        self.boss.just_engaged_with(ctx);
    }

    fn just_died(&mut self, ctx: &mut Ctx, _killer: Option<Entity>) {
        self.boss.just_died(ctx);
    }

    fn just_summoned(&mut self, ctx: &mut Ctx, summon: Entity) {
        self.boss.just_summoned(ctx, summon);
    }

    fn summoned_creature_despawn(&mut self, _ctx: &mut Ctx, summon: Entity) {
        self.boss.summoned_creature_despawn(summon);
    }

    fn update_ai(&mut self, ctx: &mut Ctx, diff_ms: u32) {
        boss_update_ai(self, ctx, diff_ms);
    }
}
