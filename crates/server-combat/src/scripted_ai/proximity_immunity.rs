//! Registered proximity releases for creatures with sourced pre-pull immunity.
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position, UnitFlags};

use crate::maps::InMap;

struct Release {
    map: u32,
    entry: u32,
    radius: f32,
    aura: u32,
}

const RELEASES: &[Release] = &[
    // NALORAKK_PROXIMITY_RELEASE_LOG_CORRELATED: WoWCombatLog-102026_223255.txt
    // lines 172137–172172 show a player about 18.7 yd away, Watchful Gaze removal,
    // and the first attack. The 20 yd trigger and its link to immunity are inferred.
    Release {
        map: 2825,
        entry: 246404,
        radius: 20.0,
        aura: 1254622,
    },
];

pub(super) fn update(world: &mut World) {
    let immunity = UnitFlags::IMMUNE_TO_PC | UnitFlags::IMMUNE_TO_NPC;
    let creatures: Vec<_> = world
        .query_filtered::<
            (Entity, &Npc, &InMap, &Position, &UnitFlags),
            Without<crate::grid_activity::GridDormant>,
        >()
        .iter(world)
        .filter_map(|(entity, npc, map, position, flags)| {
            let release = RELEASES
                .iter()
                .find(|release| release.map == map.map_id && release.entry == npc.template_id)?;
            (flags.0 & immunity != 0).then_some((entity, *map, *position, release))
        })
        .collect();
    for (creature, map, position, release) in creatures {
        if !crate::auras::has_aura(world, creature, release.aura) {
            crate::auras::apply_aura(
                world,
                &crate::auras::AuraApplication {
                    caster: creature,
                    target: creature,
                    spell_id: release.aura,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let nearby = world
            .query::<(&Player, &InMap, &Position, &Health)>()
            .iter(world)
            .any(|(_, copy, player, health)| {
                *copy == map
                    && health.current > 0.0
                    && Vec3::new(player.x, player.y, player.z)
                        .distance(Vec3::new(position.x, position.y, position.z))
                        <= release.radius
            });
        if nearby {
            world.get_mut::<UnitFlags>(creature).unwrap().0 &= !immunity;
            crate::auras::remove_auras(
                world,
                creature,
                crate::auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == release.aura,
            );
            crate::spell_cast::mark_dirty(world, creature);
        }
    }
}
