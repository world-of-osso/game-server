//! Follower target admission uses complete walk corridors or an immediately usable ranged attack.
use super::Follower;
use crate::maps::InMap;
use crate::navigation::{Goal, Navigation};
use ::navigation::worker::{Output, Query};
use bevy::prelude::*;

#[derive(Component)]
struct TargetPaths {
    origin: Vec3,
    targets: Vec<(Entity, Vec3, bool)>,
}

pub(super) fn retain_reachable(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    enemies: &mut Vec<(Entity, Option<Entity>)>,
) -> bool {
    let Some(origin) = crate::spell_cast::targets::position(world, unit) else {
        enemies.clear();
        return false;
    };
    let Some(map) = world.get::<InMap>(unit).copied() else {
        enemies.clear();
        return false;
    };
    enemies.retain(|(enemy, _)| crate::spell_cast::targets::position(world, *enemy).is_some());
    let range = attack_range(world, unit, follower);
    let pending: Vec<_> = enemies
        .iter()
        .filter_map(|(enemy, _)| {
            let target = crate::spell_cast::targets::position(world, *enemy)?;
            let ranged = range.is_some_and(|range| origin.distance(target) <= range)
                && crate::line_of_sight::sees_in_world(world, unit, *enemy);
            (!ranged).then_some((*enemy, target))
        })
        .collect();
    if pending.is_empty() {
        return false;
    }
    let cached = world.get::<TargetPaths>(unit).is_some_and(|cached| {
        cached.origin.distance(origin) <= 2.0
            && cached.targets.len() == pending.len()
            && cached
                .targets
                .iter()
                .zip(&pending)
                .all(|((a, p, _), (b, q))| a == b && p.distance(*q) <= 1.0)
    });
    if !cached {
        let query = Query::Candidates {
            from: origin,
            targets: pending.iter().map(|(_, p)| *p).collect(),
        };
        let Some(mut nav) = world.get_resource_mut::<Navigation>() else {
            error!(?unit, "Follower target selection requires navigation");
            enemies.clear();
            return false;
        };
        let result = nav.query_deferred(unit, map.map_id, Goal::FollowerTargetSelection, query);
        let paths = match result {
            None => {
                enemies.retain(|(entity, _)| {
                    !pending.iter().any(|(candidate, _)| candidate == entity)
                });
                return true;
            }
            Some(Ok(Some(Output::Candidates(paths)))) => paths,
            Some(other) => {
                error!(?unit, ?other, "Follower target navigation failed");
                enemies.clear();
                return false;
            }
        };
        let targets = pending
            .iter()
            .zip(paths)
            .map(|((entity, position), path)| (*entity, *position, path.is_ok()))
            .collect();
        world
            .entity_mut(unit)
            .insert(TargetPaths { origin, targets });
    }
    let paths = world
        .get::<TargetPaths>(unit)
        .expect("target paths just populated");
    enemies.retain(|(entity, _)| {
        !pending.iter().any(|(candidate, _)| candidate == entity)
            || paths
                .targets
                .iter()
                .any(|(candidate, _, reachable)| candidate == entity && *reachable)
    });
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_range_comes_from_action_metadata_not_the_kit_catalog() {
        let (mut harness, unit) = super::super::kit_tests::at_level_unengaged(209059, 32, |_| {});
        let world = harness.app.world_mut();
        let follower = world.get::<Follower>(unit).unwrap().clone();
        let expected = world
            .resource::<crate::spell_info::SpellStore>()
            .get(108853)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .range
            .unwrap()
            .max_hostile;
        world
            .entity_mut(unit)
            .insert(super::super::FollowerActionSpells(vec![108853]));
        world.remove_resource::<super::super::FollowerCatalog>();
        assert_eq!(attack_range(world, unit, &follower), Some(expected));
        world
            .entity_mut(unit)
            .insert(super::super::FollowerActionSpells(Vec::new()));
        assert_eq!(attack_range(world, unit, &follower), None);
    }
}

fn attack_range(world: &World, unit: Entity, follower: &Follower) -> Option<f32> {
    let store = world.get_resource::<crate::spell_info::SpellStore>()?;
    let actions = &world
        .get::<super::FollowerActionSpells>(unit)
        .expect("follower action metadata unavailable")
        .0;
    actions
        .iter()
        .copied()
        .flat_map(|id| {
            let resolved = crate::spell_cast::knowledge::resolve_actor_action(world, unit, id).ok();
            std::iter::once(id).chain(resolved)
        })
        .filter_map(|id| {
            let spell = store.get(id)?;
            (crate::spell_info::positivity::negative_effects(store, spell) != 0)
                .then_some(spell.misc.as_ref()?.range?.max_hostile)
        })
        .chain(follower.profile.melee.then_some(super::ai::MELEE_REACH))
        .filter(|range| *range > 0.0)
        .reduce(f32::max)
}
