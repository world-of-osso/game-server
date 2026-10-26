//! Age a real consumable aura without advancing the encounter or changing its tick ordinals.
use super::ActiveAuras;
use bevy::prelude::*;

pub(crate) fn age(
    world: &mut World,
    actor: Entity,
    spell: u32,
    seconds: f64,
) -> Result<f64, String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("precombat aura age must be finite and nonnegative".into());
    }
    let mut auras = world
        .get_mut::<ActiveAuras>(actor)
        .ok_or("precombat consumable has no native auras")?;
    let instance = auras
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == actor)
        .map(|aura| aura.instance_id)
        .ok_or_else(|| {
            format!("precombat consumable spell{spell} did not apply its native aura")
        })?;
    let aura = auras
        .get_mut(instance)
        .expect("owned consumable instance just resolved");
    let elapsed = seconds.min(f64::from(aura.duration_ms) / 1000.0);
    let expiry = aura
        .expires_at
        .as_mut()
        .ok_or("precombat consumable requires a finite aura")?;
    *expiry -= elapsed;
    for effect in &mut aura.effects {
        effect.periodic_started_at -= elapsed;
    }
    Ok(elapsed)
}
