//! Unit state an aura set implies, reconciled on every aura change:
//! - MOD_INVISIBILITY 18 (`HandleModInvisibility`): no player detects the invisible
//!   unit (there are no invisibility detection auras), so it leaves every client
//!   (`UpdateObjectVisibility`) and is no valid target (`CanSeeOrDetect` in
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
use crate::interest::NetworkVisibility;

/// Under a MOD_INVISIBILITY aura. `shown` records whether the unit was replicated
/// to clients, so the aura's end shows it again.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invisible {
    shown: bool,
}

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
    reconcile_invisibility(world, unit, invisible);
    reconcile_unattackable(world, unit, unattackable);
}

fn reconcile_invisibility(world: &mut World, unit: Entity, invisible: bool) {
    let current = world.get::<Invisible>(unit).copied();
    match (invisible, current) {
        (true, None) => {
            let shown = world.get::<NetworkVisibility>(unit).is_some();
            world
                .entity_mut(unit)
                .insert(Invisible { shown })
                .remove::<NetworkVisibility>();
        }
        (false, Some(Invisible { shown })) => {
            let mut entity = world.entity_mut(unit);
            entity.remove::<Invisible>();
            if shown {
                entity.insert(NetworkVisibility::default());
            }
        }
        _ => {}
    }
}

fn reconcile_unattackable(world: &mut World, unit: Entity, unattackable: bool) {
    let Some(flags) = world.get::<UnitFlags>(unit).copied() else {
        return;
    };
    if flags.is_attackable() != unattackable {
        return;
    }
    let next = if unattackable {
        flags.0 | UnitFlags::NON_ATTACKABLE_2
    } else {
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
