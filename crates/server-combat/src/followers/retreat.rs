//! Admit Vengeful Retreat only when its native landing is safe.
use bevy::prelude::*;
use shared::components::{Position, Rotation};

pub fn prepare_havoc_retreat(
    world: &mut World,
    actor: Entity,
    target: Entity,
) -> Result<bool, String> {
    let from = *world
        .get::<Position>(actor)
        .ok_or("Retreat actor lacks position")?;
    let to = *world
        .get::<Position>(target)
        .ok_or("Retreat target lacks position")?;
    let offset = Vec3::new(from.x - to.x, 0.0, from.z - to.z);
    if offset.length_squared() <= f32::EPSILON {
        return Err("Retreat actor and target share a position".into());
    }
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(198793)
        .ok_or("native Vengeful Retreat absent")?
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 144)
        .ok_or("native Vengeful Retreat knockback absent")?;
    let horizontal = effect.misc_values[0] as f32 * 0.1;
    let vertical =
        crate::spell_cast::effects::modded_base_points(world, actor, &spell, effect) * 0.1;
    let velocity = -offset.normalize() * horizontal + Vec3::Y * vertical;
    let Some(landing) = crate::knockback::projected_landing(world, actor, velocity) else {
        return Ok(false);
    };
    if super::positioning::route_enters_unpulled_sight(world, actor, target, &[landing, landing])? {
        return Ok(false);
    }
    world
        .get_mut::<Rotation>(actor)
        .ok_or("Retreat actor lacks rotation")?
        .y = offset.x.atan2(offset.z);
    Ok(true)
}
