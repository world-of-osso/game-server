//! Copy-local Voidscar opening progression and miniboss death rewards.
use bevy::prelude::*;
use shared::components::{Npc, UnitFlags};

use super::instance_script;
use crate::maps::InMap;

// INFERRED_WCL_ORDER: the chosen miniboss dies before Taz'Rah in
// h82XJKQYxVztdnqv/1 and NYqJhxcCaVZQpf8t/6. Retail gate internals are unknown.
const MAP: u32 = 2923;
const TAZ_RAH: u32 = 238887;
const OPENING_STATE: u32 = 2_675_450;
const REWARDS: &[(u32, u32)] = &[(267545, 1298903), (267546, 1298902)];
const LOCK_FLAGS: u32 = UnitFlags::IMMUNE_TO_PC | UnitFlags::NOT_SELECTABLE;

// Mythic copies fill at 23; active keystones use difficulty 8.
fn supported_difficulty(difficulty: u32) -> bool {
    matches!(difficulty, 8 | 23)
}

#[derive(Component)]
struct OpeningLock(u32);

/// Hold the first boss until either opening miniboss is defeated in this copy.
pub(super) fn update(world: &mut World) {
    let bosses: Vec<_> = world
        .query::<(Entity, &Npc, &InMap, &crate::zones::CreatureDifficulty)>()
        .iter(world)
        .filter_map(|(entity, npc, map, difficulty)| {
            (map.map_id == MAP
                && map.instance_id != 0
                && supported_difficulty(difficulty.0)
                && npc.template_id == TAZ_RAH)
                .then_some((entity, *map))
        })
        .collect();
    for (boss, map) in bosses {
        let opened = instance_script::get_data(world, map, OPENING_STATE) != 0;
        if opened {
            if let Some(lock) = world.get::<OpeningLock>(boss) {
                let added = lock.0;
                if let Some(mut flags) = world.get_mut::<UnitFlags>(boss) {
                    flags.0 &= !added;
                }
                world.entity_mut(boss).remove::<OpeningLock>();
            }
        } else if world.get::<OpeningLock>(boss).is_none() {
            let Some(mut flags) = world.get_mut::<UnitFlags>(boss) else {
                error!(?boss, "Voidscar first boss has no unit flags");
                continue;
            };
            let added = LOCK_FLAGS & !flags.0;
            flags.0 |= LOCK_FLAGS;
            world.entity_mut(boss).insert(OpeningLock(added));
        }
    }
}

/// Award the selected miniboss's DB2 spell and unlock this copy on death.
pub(crate) fn on_unit_death(world: &mut World, creature: Entity) {
    let Some(map) = world.get::<InMap>(creature).copied() else {
        return;
    };
    if map.map_id != MAP
        || map.instance_id == 0
        || !world
            .get::<crate::zones::CreatureDifficulty>(creature)
            .is_some_and(|difficulty| supported_difficulty(difficulty.0))
    {
        return;
    }
    let Some(entry) = world.get::<Npc>(creature).map(|npc| npc.template_id) else {
        return;
    };
    let Some((_, spell)) = REWARDS.iter().find(|(source, _)| *source == entry) else {
        return;
    };
    if instance_script::get_data(world, map, OPENING_STATE) != 0 {
        return;
    }
    // VERIFIED_DB2: both spells use duration row21 (-1). WCL reports the cast
    // from the dying miniboss on all five players; no 1800s expiry was observed.
    if let Err(error) = crate::spell_cast::creature_cast(world, creature, *spell, None, true) {
        error!(
            ?creature,
            spell,
            ?error,
            "Voidscar miniboss death reward failed"
        );
    }
    instance_script::set_data(world, map, OPENING_STATE, entry);
}
