//! TC a352b1fa kings_rest.cpp `go_kings_rest_serpentine_seal` (288466).
use bevy::prelude::*;

use crate::game_objects::GameObjectInUse;

pub(crate) const SCRIPT: &str = "go_kings_rest_serpentine_seal";
const NPC_SHADOW_OF_ZUL: u32 = 137020;
const SUMMON_DELAY_MS: u32 = 3_000;
/// `ShadowOfZulSerpentBossEventSpawnPosition`.
const ZUL_POSITION: [f32; 4] = [-1066.0365, 2643.302, 819.37024, 5.028946];
const SERPENT_BOSS_EVENT: &str = "SerpentBossEvent";

/// The seal's `TaskScheduler` entry summoning Shadow of Zul.
#[derive(Component)]
pub(crate) struct PendingZulSummon {
    remaining_ms: u32,
}

/// `OnGossipHello`: `UseDoorOrButton`, then summon Zul 3s later.
pub(crate) fn gossip_hello(world: &mut World, _player: Entity, seal: Entity) {
    // Template autoClose is 0, so the seal stays used for the copy's lifetime.
    world.entity_mut(seal).insert((
        GameObjectInUse,
        PendingZulSummon {
            remaining_ms: SUMMON_DELAY_MS,
        },
    ));
}

/// `GameObjectAI::UpdateAI`: the seal's scheduler.
pub(crate) fn update(world: &mut World) {
    let diff_ms = world.resource::<Time>().delta().as_millis() as u32;
    let mut due = Vec::new();
    for (seal, mut pending) in world
        .query::<(Entity, &mut PendingZulSummon)>()
        .iter_mut(world)
    {
        pending.remaining_ms = pending.remaining_ms.saturating_sub(diff_ms);
        if pending.remaining_ms == 0 {
            due.push(seal);
        }
    }
    for seal in due {
        world.entity_mut(seal).remove::<PendingZulSummon>();
        let Some(zul) = crate::zones::summon_creature(world, seal, NPC_SHADOW_OF_ZUL, ZUL_POSITION)
        else {
            continue;
        };
        // Set before the summon's first AI update runs JustAppeared.
        crate::zones::set_script_string_id(world, zul, SERPENT_BOSS_EVENT);
    }
}
