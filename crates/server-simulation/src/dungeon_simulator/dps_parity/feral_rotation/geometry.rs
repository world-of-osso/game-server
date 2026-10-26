//! Feral area facts use actual native damage-effect recipients, not copied radii.
use bevy::prelude::*;

pub(super) fn read_area_recipients(
    world: &mut World,
    actor: Entity,
    supplied: &[Entity],
    spell_id: u32,
) -> Result<usize, String> {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .cloned()
        .ok_or_else(|| format!("native Feral area spell{spell_id} absent"))?;
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == crate::spell_cast::effects::EFFECT_SCHOOL_DAMAGE)
        .ok_or_else(|| format!("native Feral area spell{spell_id} has no damage effect"))?;
    let maximum = spell
        .target_restrictions
        .as_ref()
        .map_or(0, |row| row.max_targets);
    let selected = crate::spell_cast::targets::select_targets(
        world,
        actor,
        supplied.first().copied(),
        &spell,
        effect,
        maximum,
        false,
    )
    .map_err(|target| format!("native Feral area spell{spell_id} unsupported target{target}"))?;
    Ok(selected
        .into_iter()
        .filter(|target| supplied.contains(target))
        .count())
}
