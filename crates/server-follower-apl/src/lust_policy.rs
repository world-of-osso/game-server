//! INFERRED_WCL: shared follower raid-haste timing, not a literal SimC row.
//!
//! Source: 144 Midnight S2 keys, +2..+10, 16 per level across all 8 dungeons (213 party
//! lust applications; audit, sample and derivations in /home/osso/.worktrees/lust-audit/report.md).
//! Retail spends the first lust on an early large trash pull (pull 1 in 85/123 keys) and
//! holds the next for the key's final boss (66/85 second lusts). A lockout ending mid-run is
//! not spent on trash: only 14/85 second lusts land on trash, with no repeated pull pattern.
//! Retire this inferred rule when a sourced party cooldown policy is available.
use bevy::prelude::*;
use server::class_progression::KnownSpells;
use server::followers;
use server::maps::InMap;
use server::scripted_ai::instance_script::{self, EncounterState, InstanceScripts};

use crate::literal_rotation::execution::{self, Execution};

/// p10 of live engaged enemies at 110 retail trash lusts: distinct enemy instances the party
/// damaged within 5 s of the lust.
const TRASH_ENGAGED: usize = 11;
/// p10 of seconds into the pull at the same 110 retail trash lusts.
const TRASH_DELAY: f64 = 7.7;

/// The owner's party has spent its first lust in this instance copy.
#[derive(Component)]
struct FirstLustSpent(InMap);

pub(crate) fn try_lust(
    world: &mut World,
    actor: Entity,
    targets: &[Entity],
    elapsed: f64,
) -> Result<bool, String> {
    let Some((caster, spell)) = read_lust_action(world, actor) else {
        return Ok(false);
    };
    let party = crate::follower::party_of(world, actor);
    if party
        .iter()
        .any(|&member| server::auras::bloodlust::has_lockout(world, member))
    {
        mark_first_lust_spent(world, actor);
        return Ok(false);
    }
    // SimC fixed-length fights lust at the start; parity keeps that.
    let fixed = world.contains_resource::<crate::registry::FixedLengthFight>();
    if !fixed && !is_lust_pull(world, actor, targets, elapsed) {
        return Ok(false);
    }
    let native = world
        .resource::<server::spell_info::SpellStore>()
        .get(spell)
        .cloned()
        .ok_or_else(|| format!("native lust spell {spell} is missing"))?;
    if !followers::action_safe(world, caster, caster, Some(spell))
        || server::spell_cast::read_native_cast_start(world, caster, &native, caster).is_err()
    {
        return Ok(false);
    }
    execution::execute(
        world,
        caster,
        Execution::Spell(spell, caster, targets.first().copied()),
    )?;
    mark_first_lust_spent(world, actor);
    Ok(true)
}

/// The actor's own raid-haste spell, or its Ferocity pet's Primal Rage.
fn read_lust_action(world: &World, actor: Entity) -> Option<(Entity, u32)> {
    let own = world.get::<KnownSpells>(actor).and_then(|known| {
        [2825, 32182, 80353, 390386]
            .into_iter()
            .find(|&spell| known.contains(spell))
    });
    if let Some(spell) = own {
        return Some((actor, spell));
    }
    const PRIMAL_RAGE: u32 = 264667;
    let pet = server::hunter_pet::summoned_pet(world, actor)?;
    server::hunter_pet::pet_has_specialization_spell(world, pet, PRIMAL_RAGE)
        .then_some((pet, PRIMAL_RAGE))
}

/// The final boss at any point of its pull (retail final-boss lusts: p10 0.7 s, median
/// 4.0 s; 11/23 later ones waited for a party lockout), else the first large trash pull.
fn is_lust_pull(world: &mut World, actor: Entity, targets: &[Entity], elapsed: f64) -> bool {
    if targets.iter().any(|&target| is_final_boss(world, target)) {
        return true;
    }
    let trash = !targets
        .iter()
        .any(|&target| followers::is_boss(world, target));
    trash
        && !first_lust_spent(world, actor)
        && elapsed >= TRASH_DELAY
        && targets.len() >= TRASH_ENGAGED
}

/// A boss whose instance copy has every other bound boss done.
fn is_final_boss(world: &mut World, target: Entity) -> bool {
    if !followers::is_boss(world, target) {
        return false;
    }
    let (Some(map), Some(entry)) = (
        world.get::<InMap>(target).copied(),
        world
            .get::<shared::components::Npc>(target)
            .map(|npc| npc.template_id),
    ) else {
        return false;
    };
    // The public boss-state read creates the copy's instance script when it is missing.
    instance_script::get_boss_state(world, map, 0);
    let Some(script) = world
        .get_resource::<InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&map))
    else {
        return false;
    };
    let Some(target_boss) = script
        .boss_creatures
        .iter()
        .find_map(|&(creature, boss)| (creature == entry).then_some(boss))
    else {
        return false;
    };
    script
        .boss_creatures
        .iter()
        .filter(|&&(_, boss)| boss != target_boss)
        .all(|&(_, boss)| {
            script
                .bosses
                .get(boss as usize)
                .is_some_and(|info| info.state == EncounterState::Done)
        })
}

fn first_lust_spent(world: &World, actor: Entity) -> bool {
    let map = world.get::<InMap>(actor).copied();
    followers::acting_follower(world, actor)
        .and_then(|follower| world.get::<FirstLustSpent>(follower.owner))
        .is_some_and(|spent| Some(spent.0) == map)
}

/// Record on the party owner, for this copy, that a lust has been applied to the party.
fn mark_first_lust_spent(world: &mut World, actor: Entity) {
    let (Some(map), Some(follower)) = (
        world.get::<InMap>(actor).copied(),
        followers::acting_follower(world, actor),
    ) else {
        return;
    };
    world.entity_mut(follower.owner).insert(FirstLustSpent(map));
}
