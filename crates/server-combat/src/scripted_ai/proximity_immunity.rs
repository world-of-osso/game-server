//! Registered proximity releases for creatures with sourced pre-pull immunity.
use bevy::prelude::*;
use shared::components::{Health, Npc, Player, Position, UnitFlags};

use crate::maps::InMap;

struct Release {
    map: u32,
    entry: u32,
    radius: f32,
    aura: Option<u32>,
}

const RELEASES: &[Release] = &[
    // NALORAKK_PROXIMITY_RELEASE_LOG_CORRELATED: WoWCombatLog-102026_223255.txt
    // lines 172137–172172 show a player about 18.7 yd away, Watchful Gaze removal,
    // and the first attack. The 20 yd trigger and its link to immunity are inferred.
    Release {
        map: 2825,
        entry: 246404,
        radius: 20.0,
        aura: Some(1254622),
    },
    // GEEZLE_PROXIMITY_RELEASE_LOG_CORRELATED: template unit_flags 0x340 are
    // immune; TC 15e0ed76 has no Floodgate script. The radius is the largest
    // nearest-player distance at the first hostile cast in WCL
    // 67c3WanVygd4ZvCt/1 pull 13 (Taunt), CPzKQd3xLrWJgRNw/1 pull 14 and
    // hAtcnVyGZ6JRvWpN/3 pull 12. The Retail trigger itself is not logged.
    Release {
        map: 2773,
        entry: 226404,
        radius: 21.6577,
        aura: None,
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
        if let Some(aura) = release.aura
            && !crate::auras::has_aura(world, creature, aura)
        {
            crate::auras::apply_aura(
                world,
                &crate::auras::AuraApplication {
                    caster: creature,
                    target: creature,
                    spell_id: aura,
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
            if let Some(spell_id) = release.aura {
                crate::auras::remove_auras(
                    world,
                    creature,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == spell_id,
                );
            }
            crate::spell_cast::mark_dirty(world, creature);
        }
    }
}
