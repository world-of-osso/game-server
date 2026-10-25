//! Observed destination-directed hostile cone104; native targeting decides party exposure.
use super::{Follower, ai, tank_planner};
use crate::maps::InMap;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Position};
use std::collections::BTreeSet;

const DESTINATION_CONE: u32 = 104;

#[derive(Component, Clone, Copy)]
struct ObservedFrontal {
    caster: Entity,
    spell_id: u32,
}

/// Ownership of the existing point component, not permission to replace another producer.
#[derive(Component)]
struct FrontalPosition;

struct FrontalProjection<'a> {
    tank: Entity,
    caster: Entity,
    spell: SpellInfo,
    party: &'a [Entity],
}

fn directional_effects(spell: &SpellInfo) -> impl Iterator<Item = &SpellEffect> {
    spell
        .effects
        .iter()
        .filter(|effect| effect.implicit_targets.contains(&DESTINATION_CONE))
}

/// Capture before any follower's legitimate kick can end the source cast.
pub(super) fn record_observed_frontal(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
) {
    let engaged = ai::engaged_enemies(world, follower, tank, party);
    for &(caster, _) in &engaged {
        let Some(cast) = world.get::<CastState>(caster) else {
            continue;
        };
        if cast.target != tank.to_bits() {
            continue;
        }
        let Some(spell) = world.resource::<SpellStore>().get(cast.spell_id) else {
            continue;
        };
        if directional_effects(spell).next().is_some() {
            let observed = ObservedFrontal {
                caster,
                spell_id: cast.spell_id,
            };
            world.entity_mut(tank).insert(observed);
            return;
        }
    }
    let still_engaged = world
        .get::<ObservedFrontal>(tank)
        .is_some_and(|known| engaged.iter().any(|(enemy, _)| *enemy == known.caster));
    if !still_engaged {
        world.entity_mut(tank).remove::<ObservedFrontal>();
    }
}

pub(super) fn update_goal(world: &mut World, tank: Entity, follower: &Follower, party: &[Entity]) {
    if follower.profile.role != shared::group::GroupRole::Tank {
        return;
    }
    if world.get::<tank_planner::AdmittedPosition>(tank).is_some()
        && world.get::<FrontalPosition>(tank).is_none()
    {
        return;
    }
    match read_goal(world, tank, follower, party) {
        Ok(Some(point)) => {
            world
                .entity_mut(tank)
                .insert((tank_planner::AdmittedPosition(point), FrontalPosition));
            let selected = world
                .get::<crate::networking::CombatTarget>(tank)
                .and_then(|target| target.0);
            tank_planner::apply_admitted_position(world, tank, selected);
        }
        Ok(None) => clear_owned_position(world, tank),
        Err(error) => {
            debug!(?tank, %error, "Tank frontal position unavailable");
            clear_owned_position(world, tank);
        }
    }
}

fn clear_owned_position(world: &mut World, tank: Entity) {
    if world.entity_mut(tank).take::<FrontalPosition>().is_some() {
        world
            .entity_mut(tank)
            .remove::<tank_planner::AdmittedPosition>();
    }
}

fn read_projection<'a>(
    world: &World,
    tank: Entity,
    party: &'a [Entity],
) -> Result<Option<FrontalProjection<'a>>, String> {
    let Some(known) = world.get::<ObservedFrontal>(tank).copied() else {
        return Ok(None);
    };
    let victim = world
        .get::<crate::networking::CombatTarget>(known.caster)
        .and_then(|target| target.0);
    let casting_elsewhere = world
        .get::<CastState>(known.caster)
        .is_some_and(|cast| cast.target != tank.to_bits());
    if victim != Some(tank)
        || casting_elsewhere
        || crate::auras::has_breakable_by_damage_cc(world, known.caster)
    {
        return Ok(None);
    }
    let spell = world
        .resource::<SpellStore>()
        .get(known.spell_id)
        .ok_or("Observed frontal lacks native spell metadata")?
        .clone();
    Ok(Some(FrontalProjection {
        tank,
        caster: known.caster,
        spell,
        party,
    }))
}

fn read_goal(
    world: &mut World,
    tank: Entity,
    follower: &Follower,
    party: &[Entity],
) -> Result<Option<Vec3>, String> {
    let Some(projection) = read_projection(world, tank, party)? else {
        return Ok(None);
    };
    let here =
        crate::spell_cast::targets::position(world, tank).ok_or("Frontal tank lacks position")?;
    if projection.read_targets(world, here)?.is_empty() {
        return Ok(None);
    }
    let Some(point) = projection.read_safe_pose(world)? else {
        return Ok(None);
    };
    if tank_planner::read_movement_admission(world, tank, projection.caster, follower, Some(point))?
        != Some(true)
    {
        return Ok(None);
    }
    Ok(Some(point))
}

impl FrontalProjection<'_> {
    fn read_targets(
        &self,
        world: &mut World,
        destination: Vec3,
    ) -> Result<BTreeSet<Entity>, String> {
        let mut spell = self.spell.clone();
        spell.cast_destination = Some([destination.x, destination.y, destination.z]);
        let mut exposed = BTreeSet::new();
        for effect in directional_effects(&spell) {
            let targets = crate::spell_cast::targets::select_targets(
                world,
                self.caster,
                Some(self.tank),
                &spell,
                effect,
                0,
                true,
            )
            .map_err(|selector| format!("Native frontal selector {selector} is unavailable"))?;
            exposed.extend(
                targets
                    .into_iter()
                    .filter(|target| *target != self.tank && self.party.contains(target)),
            );
        }
        Ok(exposed)
    }

    fn read_direction(&self, world: &mut World) -> Result<Option<Vec3>, String> {
        let center = crate::spell_cast::targets::position(world, self.caster)
            .ok_or("Frontal caster lacks position")?;
        let mut angles = Vec::new();
        for &member in self.party {
            if member == self.tank {
                continue;
            }
            let health = world
                .get::<Health>(member)
                .ok_or("Frontal party health is unavailable")?;
            if health.current <= 0.0 {
                continue;
            }
            let point = crate::spell_cast::targets::position(world, member)
                .ok_or("Frontal party position is unavailable")?;
            if self.read_targets(world, point)?.contains(&member) {
                let offset = point - center;
                angles.push(offset.z.atan2(offset.x));
            }
        }
        Ok(widest_gap_direction(angles))
    }

    fn read_safe_pose(&self, world: &mut World) -> Result<Option<Vec3>, String> {
        let Some(direction) = self.read_direction(world)? else {
            return Ok(None);
        };
        let point = self.read_grounded_candidate(world, direction)?;
        if !self.read_targets(world, point)?.is_empty() {
            return Ok(None);
        }
        let position = Position {
            x: point.x,
            y: point.y,
            z: point.z,
        };
        if !crate::line_of_sight::mutually_visible_at(world, self.tank, position, self.caster)? {
            return Ok(None);
        }
        Ok(Some(point))
    }

    fn read_grounded_candidate(&self, world: &mut World, direction: Vec3) -> Result<Vec3, String> {
        let center = crate::spell_cast::targets::position(world, self.caster)
            .ok_or("Frontal caster lacks position")?;
        let map = *world
            .get::<InMap>(self.tank)
            .ok_or("Frontal tank lacks map")?;
        let candidate = center + direction * ai::MELEE_REACH;
        let mut ground = world
            .get_resource_mut::<crate::ground::GroundMap>()
            .ok_or("Frontal point requires native ground")?;
        let crate::ground::GroundProbe::Supported(y) = ground.creature_ground_in(map, candidate)
        else {
            return Err("Frontal candidate lacks actual floor support".into());
        };
        Ok(Vec3::new(candidate.x, y, candidate.z))
    }
}

fn widest_gap_direction(mut angles: Vec<f32>) -> Option<Vec3> {
    if angles.is_empty() {
        return None;
    }
    angles.sort_by(f32::total_cmp);
    let (_, angle) = angles
        .iter()
        .enumerate()
        .map(|(index, &left)| {
            let right = if index + 1 == angles.len() {
                angles[0] + std::f32::consts::TAU
            } else {
                angles[index + 1]
            };
            let gap = right - left;
            (gap, left + gap * 0.5)
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .unwrap();
    Some(Vec3::new(angle.cos(), 0.0, angle.sin()))
}
