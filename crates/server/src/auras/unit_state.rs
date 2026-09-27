//! Unit state an aura set implies, reconciled on every aura change:
//! - MOD_INVISIBILITY 18 (`HandleModInvisibility`): no player detects the invisible
//!   unit (there are no invisibility detection auras), so interest hides it from
//!   every client (`UpdateObjectVisibility`) and it is no valid target (`CanSeeOrDetect` in
//!   `IsValidAttackTarget`/`IsValidAssistTarget`, `spell_cast::targets`); the last
//!   aura's removal shows it again.
//! - MOD_UNATTACKABLE 93 (`HandleModUnattackable`): `UNIT_FLAG_NON_ATTACKABLE_2`
//!   while any such aura is on; applying it stops the unit's attack in a dungeon
//!   (`AttackStop`) and its whole combat elsewhere (`CombatStop`).

use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::UnitFlags;

use super::ActiveAuras;
use super::handlers::{MOD_INVISIBILITY, MOD_UNATTACKABLE};

/// Under a MOD_INVISIBILITY aura (hidden in `interest`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invisible;

/// Under a MOD_UNATTACKABLE aura: the aura set `NON_ATTACKABLE_2`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct Unattackable;

fn has_type(auras: &ActiveAuras, aura_type: u32) -> bool {
    auras
        .iter()
        .flat_map(|aura| &aura.effects)
        .any(|effect| effect.aura_type == aura_type)
}

pub(super) fn reconcile(world: &mut World, unit: Entity) {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return;
    };
    let invisible = has_type(auras, MOD_INVISIBILITY);
    let unattackable = has_type(auras, MOD_UNATTACKABLE);
    if invisible != world.get::<Invisible>(unit).is_some() {
        if invisible {
            world.entity_mut(unit).insert(Invisible);
        } else {
            world.entity_mut(unit).remove::<Invisible>();
        }
        crate::interest::set_hidden(world, unit, invisible);
    }
    if unattackable != world.get::<Unattackable>(unit).is_some() {
        reconcile_unattackable(world, unit, unattackable);
    }
}

/// The first MOD_UNATTACKABLE aura sets the flag, the last one's removal clears it.
fn reconcile_unattackable(world: &mut World, unit: Entity, unattackable: bool) {
    let flags = world
        .get::<UnitFlags>(unit)
        .copied()
        .unwrap_or(UnitFlags(0));
    let next = if unattackable {
        world.entity_mut(unit).insert(Unattackable);
        flags.0 | UnitFlags::NON_ATTACKABLE_2
    } else {
        world.entity_mut(unit).remove::<Unattackable>();
        flags.0 & !UnitFlags::NON_ATTACKABLE_2
    };
    world.entity_mut(unit).insert(UnitFlags(next));
    if let Some(dirty) = world.get_resource::<ReplicationDirtyQueue>() {
        dirty.mark(unit);
    }
    if !unattackable {
        return;
    }
    let in_dungeon = world
        .get::<crate::maps::InMap>(unit)
        .is_some_and(|map| map.instance_id != 0);
    if in_dungeon {
        crate::scripted_ai::attack_stop(world, unit);
    } else {
        crate::combat::combat_stop(world, unit);
    }
}
