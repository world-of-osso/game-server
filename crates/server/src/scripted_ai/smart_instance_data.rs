//! SMART_ACTION_SET_INST_DATA34: Wailing Caverns' SetBossState mode.
//! TC a352b1fa wailing_caverns.h / instance_wailing_caverns.cpp ordinals are
//! not DB2 bits or the compact slots of the runtime's compatibility catalog.
use super::super::Ctx;
use super::super::instance_script::{self, EncounterState};

const WAILING_CAVERNS: u32 = 43;
const SET_BOSS_STATE: u32 = 1;
/// Source DATA_LORD_COBRAHN, PYTHAS, ANACONDRA, SERPENTIS, MUTANUS, KRESH, SKUM, VERDAN.
const SOURCE_ENCOUNTERS: [u32; 8] = [586, 588, 585, 590, 592, 587, 589, 591];

pub(super) fn apply(ctx: &mut Ctx, params: [u32; 7]) -> bool {
    let [data, value, mode, ..] = params;
    let Some(map) = ctx.map() else {
        bevy::log::warn!("SMART_ACTION_SET_INST_DATA: actor {} has no map", ctx.me);
        return false;
    };
    if map.map_id != WAILING_CAVERNS || mode != SET_BOSS_STATE {
        bevy::log::warn!(
            "SMART_ACTION_SET_INST_DATA: map {} mode {mode} is unconverted",
            map.map_id
        );
        return false;
    }
    let Some(state) = decode_state(value) else {
        bevy::log::warn!("SMART_ACTION_SET_INST_DATA: state {value} is unconverted");
        return false;
    };
    let Some(boss) = find_source_boss(ctx.world, map, data) else {
        bevy::log::warn!(
            "SMART_ACTION_SET_INST_DATA: map {map:?} source data {data} has no bound encounter"
        );
        return false;
    };
    // TC ignores SetBossState's result; a repeated DONE action is still executed.
    instance_script::set_boss_state(ctx.world, map, boss, state);
    true
}

fn decode_state(value: u32) -> Option<EncounterState> {
    Some(match value {
        0 => EncounterState::NotStarted,
        1 => EncounterState::InProgress,
        2 => EncounterState::Fail,
        3 => EncounterState::Done,
        _ => return None,
    })
}

fn find_source_boss(
    world: &mut bevy::prelude::World,
    map: crate::maps::InMap,
    data: u32,
) -> Option<u32> {
    let encounter = *SOURCE_ENCOUNTERS.get(data as usize)?;
    instance_script::boss_id_for_encounter(world, map, encounter)
}
