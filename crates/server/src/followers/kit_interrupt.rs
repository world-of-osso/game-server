//! Source-valid interrupt casts only. Candidate ordering and navigation belong
//! to the shared planner; ordinary spell execution owns any native charge.
use bevy::prelude::*;
use shared::components::Level;

use super::kit::{Entry, Slot};
use super::{FollowerCatalog, Profile};
use crate::spell_info::SpellStore;

const SPELL_EFFECT_INTERRUPT_CAST: u32 = 68;
const DEMONOLOGY_SPEC: u32 = 266;

/// `true` means an ordinary source-valid interrupt cast was accepted, not that
/// the hit is guaranteed to land. Never retarget, pursue or alter navigation.
pub(super) fn try_interrupt_candidates(
    world: &mut World,
    unit: Entity,
    profile: &Profile,
    candidates: &[Entity],
) -> bool {
    let spells = read_source_interrupt_spells(world, unit, profile);
    for &target in candidates {
        if crate::auras::has_breakable_by_damage_cc(world, target) {
            continue;
        }
        let accepted = match spells.as_deref() {
            Some(spells) if profile.spec_id != DEMONOLOGY_SPEC => {
                crate::spell_cast::can_interrupt_cast(world, unit, target)
                    && try_source_spells(world, unit, target, spells)
            }
            // Explicit core compatibility for absent spec rows. Demonology's
            // interrupt remains its actual learned pet spell, never a player grant.
            _ => interrupt_enemy(world, unit, target, profile.spec_id),
        };
        if accepted {
            return true;
        }
    }
    false
}

/// None means no spec kit rows (the requested legacy compatibility boundary).
/// Some(empty) means the spec exists but no source-level interrupt is available.
fn read_source_interrupt_spells(
    world: &World,
    unit: Entity,
    profile: &Profile,
) -> Option<Vec<u32>> {
    let catalog = world.get_resource::<FollowerCatalog>()?;
    let entries: Vec<_> = catalog
        .kits
        .iter()
        .filter(|entry| entry.spec_id == profile.spec_id)
        .collect();
    if entries.is_empty() {
        return None;
    }
    let level = world.get::<Level>(unit).map_or(0, |level| level.0);
    let learned = entries
        .into_iter()
        .filter(|entry| entry.available(profile.spec_id, level));
    let store = world.resource::<SpellStore>();
    let mut spells: Vec<_> = learned
        .filter(|entry| is_interrupt_entry(entry, store))
        .map(|entry| (entry.slot != Slot::Interrupt, entry.spell_id))
        .collect();
    // Stable partition preserves the source table's priority/ID ordering, but
    // explicit kicks (Rebuke) precede damage-spell interrupts (Avenger's Shield).
    spells.sort_by_key(|(secondary, _)| *secondary);
    Some(spells.into_iter().map(|(_, spell)| spell).collect())
}

fn is_interrupt_entry(entry: &Entry, store: &SpellStore) -> bool {
    match entry.slot {
        Slot::Interrupt => true,
        Slot::Attack => store.get(entry.spell_id).is_some_and(|spell| {
            spell
                .effects
                .iter()
                .any(|effect| effect.effect == SPELL_EFFECT_INTERRUPT_CAST)
        }),
        _ => false,
    }
}

fn try_source_spells(world: &mut World, unit: Entity, target: Entity, spells: &[u32]) -> bool {
    for &spell in spells {
        match crate::spell_cast::creature_cast_interrupting_self(world, unit, spell, Some(target)) {
            Ok(()) => return true,
            Err(error) => debug!(
                ?unit,
                ?target,
                spell,
                ?error,
                "Follower source interrupt unavailable"
            ),
        }
    }
    false
}

// Exact legacy bridge relocation, staged test-only until the shared caller lands.
/// Approved player-spec bridge, not authored NPC kits. IDs/names and effect68
/// are verified in local SpellName/SpellEffect12.1.5.70448 (Skull Bash's
/// effect68 child93985 is referenced by its source description). Holy/Restoration
/// have no selected kick; Demonology's pet interrupt is not a player cast.
fn interrupt_spell(spec: u32) -> Option<u32> {
    match spec {
        66 => Some(96231),    // Protection Paladin: Rebuke.
        73 => Some(6552),     // Protection Warrior: Pummel.
        63 => Some(2139),     // Fire Mage: Counterspell.
        262 => Some(57994),   // Elemental Shaman: Wind Shear.
        253 => Some(147362),  // Beast Mastery Hunter: Counter Shot.
        1467 => Some(351338), // Devastation Evoker: Quell.
        269 => Some(116705),  // Windwalker Monk: Spear Hand Strike.
        103 => Some(106839),  // Feral Druid: Skull Bash.
        _ => None,
    }
}

fn interrupt_enemy(world: &mut World, unit: Entity, enemy: Entity, spec: u32) -> bool {
    let Some((caster, spell)) = interrupt_source(world, unit, spec) else {
        return false;
    };
    if !crate::spell_cast::can_interrupt_cast(world, caster, enemy) {
        return false;
    }
    if caster != unit && super::hazards::target_is_hazardous(world, caster, caster) {
        return false;
    }
    match crate::spell_cast::creature_cast_interrupting_self(world, caster, spell, Some(enemy)) {
        Ok(()) => true,
        Err(failure) => {
            debug!("Follower {unit} interrupt {spell} at {enemy}: {failure:?}");
            false
        }
    }
}

/// CreatureFamily29/SkillLine761 teaches Felguard Axe Toss89766 at17; source
/// SpellEffect64 triggers the real347008 effect68. Never cast an unlearned pet spell.
fn interrupt_source(world: &World, unit: Entity, spec: u32) -> Option<(Entity, u32)> {
    if spec != 266 {
        return interrupt_spell(spec).map(|spell| (unit, spell));
    }
    let pet = crate::hunter_pet::summoned_pet(world, unit)?;
    let charm = world.get::<crate::pet_control::PetCharm>(pet)?;
    charm.spell_slot(89766).map(|_| (pet, 89766))
}
