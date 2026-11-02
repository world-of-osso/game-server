//! INFERRED_WCL: shared follower raid-haste timing, not a literal SimC row.
use bevy::prelude::*;
use server::class_progression::KnownSpells;
use server::followers;

use crate::literal_rotation::execution::{self, Execution};

/// Current Midnight S2 high-key samples cast on trash, including after a long
/// pull: bVvgmyZ1FM6p9C8R fight1 pulls1/5/8; GVT6rBFfJ7bDQc2M fight9
/// pulls1/3/5; xtGY7CAgR2yJLPXd fight7 pulls1/4/7. Three is the smallest
/// nearby damaged-target count observed; that count is only a pull-size proxy.
/// Retire this inferred rule when a sourced party cooldown policy is available.
pub(crate) fn try_lust(
    world: &mut World,
    actor: Entity,
    targets: &[Entity],
    elapsed: f64,
) -> Result<bool, String> {
    let spell = world.get::<KnownSpells>(actor).and_then(|known| {
        [2825, 32182, 80353, 390386]
            .into_iter()
            .find(|&spell| known.contains(spell))
    });
    let Some(spell) = spell else { return Ok(false) };
    let party = crate::follower::party_of(world, actor);
    if party
        .iter()
        .any(|&member| server::auras::bloodlust::has_lockout(world, member))
    {
        return Ok(false);
    }
    let fixed = world.contains_resource::<crate::registry::FixedLengthFight>();
    let boss = elapsed <= 10.0
        && targets
            .iter()
            .any(|&target| followers::is_boss(world, target));
    let large = elapsed >= 7.0 && targets.len() >= 3;
    if !(fixed || boss || large) {
        return Ok(false);
    }
    let native = world
        .resource::<server::spell_info::SpellStore>()
        .get(spell)
        .cloned()
        .ok_or_else(|| format!("native lust spell {spell} is missing"))?;
    if !followers::action_safe(world, actor, actor, Some(spell))
        || server::spell_cast::read_native_cast_start(world, actor, &native, actor).is_err()
    {
        return Ok(false);
    }
    execution::execute(
        world,
        actor,
        Execution::Spell(spell, actor, targets.first().copied()),
    )?;
    Ok(true)
}
