//! TC a352b1fa Battleground.cpp:100–103,334–445 and each map's OnStart.
//! Placements/models remain world.db data; Seething Shore's sourced doors have no
//! placements in the imported data, so none are fabricated here.
use bevy::prelude::*;
use shared::protocol::{GameObjectAnimation, GameObjectInfo};

use crate::game_objects::{GO_STATE_ACTIVE, GameObjectInUse, set_go_state};
use crate::maps::InMap;

const PREPARATION_SECS: f64 = 120.0;
const SECOND_STAGE_ELAPSED: f64 = 60.0;
const THIRD_STAGE_ELAPSED: f64 = 90.0;
const KOTMOGU_EARLY_GATE_DELAY: f64 = 24.0;
const CLOSED_ANIMATION: u16 = 147;
const OPEN_ANIMATION: u16 = 148;

#[derive(Debug, Default)]
pub(super) struct Preparation {
    started_at: Option<f64>,
    stage: u8,
    third_stage_at: Option<f64>,
    engaged_at: Option<f64>,
}

impl Preparation {
    /// First actual arrival starts the source clock; later arrivals cannot reset it.
    /// Existing authored roster readiness remains a separate admission requirement.
    pub(super) fn advance(&mut self, now: f64, arrived: bool, roster_ready: bool) -> bool {
        let Some(started) = self.started_at else {
            if arrived {
                self.started_at = Some(now);
                self.stage = 1;
            }
            return false;
        };
        let elapsed = now - started;
        match self.stage {
            1 if elapsed >= SECOND_STAGE_ELAPSED => self.stage = 2,
            2 if elapsed >= THIRD_STAGE_ELAPSED => {
                self.stage = 3;
                self.third_stage_at = Some(now);
            }
            3 if elapsed >= PREPARATION_SECS && roster_ready => {
                self.stage = 4;
                self.engaged_at = Some(now);
                return true;
            }
            _ => {}
        }
        false
    }
}

#[derive(Clone, Copy)]
enum GateAction {
    OpenAndDespawn(f64),
    Open,
    Remove,
    OpenAfterThirdStage,
}
use GateAction::{Open, OpenAfterThirdStage, OpenAndDespawn, Remove};

// Map scripts' OnStart door sets, not destructible objective gates. All entries
// have authored templates; loader inclusion does not invent missing placements.
const GATES: &[(u32, u32, GateAction)] = &[
    // WarsongGulch/battleground_warsong_gulch.cpp:197–207.
    (2106, 309704, OpenAndDespawn(3.0)),
    (2106, 309705, OpenAndDespawn(3.0)),
    (2106, 309883, OpenAndDespawn(3.0)),
    (2106, 242273, OpenAndDespawn(3.0)),
    (2106, 352709, OpenAndDespawn(3.0)),
    (2106, 352710, OpenAndDespawn(3.0)),
    // ArathiBasin/arathi_basin.cpp:336–354 (start event).
    (2107, 322273, OpenAndDespawn(3.0)),
    (2107, 322274, OpenAndDespawn(3.0)),
    (2107, 180322, Remove),
    // TwinPeaks/battleground_twin_peaks.cpp:174–184.
    (726, 208205, OpenAndDespawn(3.0)),
    (726, 208207, OpenAndDespawn(3.0)),
    (726, 208206, OpenAndDespawn(3.0)),
    (726, 203710, OpenAndDespawn(3.0)),
    (726, 206653, OpenAndDespawn(3.0)),
    (726, 206654, OpenAndDespawn(3.0)),
    (726, 206655, OpenAndDespawn(3.0)),
    (726, 180322, OpenAndDespawn(3.0)),
    // BattleForGilneas/battleground_battle_for_gilneas.cpp:175–186.
    (761, 207177, OpenAndDespawn(3.0)),
    (761, 207178, OpenAndDespawn(3.0)),
    (761, 205495, OpenAndDespawn(3.0)),
    (761, 205496, OpenAndDespawn(3.0)),
    // EyeOfTheStorm/battleground_eye_of_the_storm.cpp:318–331.
    (566, 184719, OpenAndDespawn(3.0)),
    (566, 184720, OpenAndDespawn(3.0)),
    // AlteracValley/battleground_alterac_valley.cpp:736–753.
    (30, 180424, OpenAndDespawn(3.0)),
    (30, 180322, Remove),
    // IsleOfConquest/battleground_isle_of_conquest.cpp:338–376.
    (628, 195703, OpenAndDespawn(20.0)),
    (628, 195491, OpenAndDespawn(20.0)),
    (628, 195436, Open),
    (628, 195437, Open),
    // TempleOfKotmogu/battleground_temple_of_kotmogu.cpp:240–254.
    (998, 215760, OpenAfterThirdStage),
    (998, 215787, Open),
    // SilvershardMines/battleground_silvershard_mines.cpp:278–288.
    (727, 212939, OpenAndDespawn(3.0)),
    (727, 212940, OpenAndDespawn(3.0)),
    (727, 212941, OpenAndDespawn(3.0)),
    (727, 212942, OpenAndDespawn(3.0)),
    // SeethingShore/battleground_seething_shore.cpp:223–232 (no placements).
    (1803, 281224, OpenAndDespawn(3.0)),
    (1803, 281226, OpenAndDespawn(3.0)),
    // DeephaulRavine/battleground_deephaul_ravine.cpp:298–305.
    (2656, 437274, OpenAndDespawn(3.0)),
    (2656, 446147, OpenAndDespawn(3.0)),
];

pub(crate) fn gate_entries(map_id: u32) -> impl Iterator<Item = u32> {
    GATES
        .iter()
        .filter_map(move |&(map, entry, _)| (map == map_id).then_some(entry))
}

fn gate_action(map_id: u32, entry: u32) -> Option<GateAction> {
    GATES
        .iter()
        .find(|&&(map, id, _)| map == map_id && id == entry)
        .map(|&(_, _, action)| action)
}

pub(super) fn prepare_map(world: &mut World, map: InMap) {
    let mut query = world.query::<(Entity, &InMap, &GameObjectInfo)>();
    let gates: Vec<_> = query
        .iter(world)
        .filter(|(_, copy, info)| **copy == map && gate_action(map.map_id, info.entry).is_some())
        .map(|(entity, _, _)| entity)
        .collect();
    for gate in gates {
        world.entity_mut(gate).insert(GameObjectAnimation {
            animation_id: CLOSED_ANIMATION,
            looping: true,
        });
    }
}

pub(super) fn update_gates(world: &mut World, map: InMap, preparation: &Preparation, now: f64) {
    let mut query = world.query::<(Entity, &InMap, &GameObjectInfo)>();
    let gates: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, copy, info)| {
            if *copy != map {
                return None;
            }
            gate_action(map.map_id, info.entry).map(|action| (entity, action))
        })
        .collect();
    for (gate, action) in gates {
        update_gate(world, gate, action, preparation, now);
    }
}

fn update_gate(
    world: &mut World,
    gate: Entity,
    action: GateAction,
    preparation: &Preparation,
    now: f64,
) {
    let opened_at = match action {
        OpenAfterThirdStage => preparation
            .third_stage_at
            .map(|at| at + KOTMOGU_EARLY_GATE_DELAY),
        _ => preparation.engaged_at,
    };
    let Some(opened_at) = opened_at.filter(|&at| now >= at) else {
        return;
    };
    let remove = match action {
        Remove => true,
        OpenAndDespawn(delay) => now - opened_at >= delay,
        _ => false,
    };
    if remove {
        world.despawn(gate);
        return;
    }
    if world.get::<GameObjectInUse>(gate).is_some() {
        return;
    }
    set_go_state(world, gate, GO_STATE_ACTIVE);
    world.entity_mut(gate).insert((
        GameObjectInUse,
        GameObjectAnimation {
            animation_id: OPEN_ANIMATION,
            looping: false,
        },
    ));
}
