//! INFERRED_WCL: drink from the class loadout while the healer-mana gate holds a pull.
use bevy::prelude::*;
use shared::components::{CombatStatus, CreatureMotion, Mana, Position, StandState};
use shared::group::GroupRole;
use shared::item_data::ItemDefinitionSource;

use super::pull_admission::MIN_READY_HEALER_MANA_FRACTION;
use super::pull_budget::{Decision, Reason};

/// Springrunner Sparkling; its ItemEffect provides Drink 1291791.
#[derive(Component)]
pub(super) struct RecoveryDrinkItem(pub u32);

#[derive(Component)]
struct DrinkingAt(Position);

pub(super) fn install(world: &mut World, unit: Entity) {
    world.entity_mut(unit).insert(RecoveryDrinkItem(260260));
}

/// Called at the actual pull-admission boundary, not by the budget reader.
pub(super) fn on_decision(world: &mut World, tank: Entity, decision: &Decision) {
    let Some(follower) = world.get::<super::Follower>(tank) else {
        return;
    };
    let owner = follower.owner;
    let Some((_, members, owner_role)) = super::party::read_admission(world, owner) else {
        return;
    };
    let members: Vec<_> = members.iter().copied().chain([owner]).collect();
    let healer_low = members.iter().copied().any(|unit| {
        let role = if unit == owner {
            owner_role == GroupRole::Healer
        } else {
            world
                .get::<super::Follower>(unit)
                .is_some_and(|f| f.profile.role == GroupRole::Healer)
        };
        role && below_gate(world, unit)
    });
    if !matches!(decision, Decision::Wait(Reason::Recovery)) || !healer_low {
        return;
    }
    for unit in members {
        if let Err(error) = try_drink(world, unit) {
            warn!(?unit, %error, "Follower recovery drink unavailable");
        }
    }
}

fn below_gate(world: &World, unit: Entity) -> bool {
    world.get::<Mana>(unit).is_some_and(|mana| {
        mana.max > 0.0 && mana.current < mana.max * MIN_READY_HEALER_MANA_FRACTION
    })
}

fn try_drink(world: &mut World, unit: Entity) -> Result<(), String> {
    let Some(item) = world.get::<RecoveryDrinkItem>(unit).map(|item| item.0) else {
        return Ok(());
    };
    if !below_gate(world, unit)
        || world.get::<DrinkingAt>(unit).is_some()
        || world
            .get::<CombatStatus>(unit)
            .is_some_and(|status| status.0)
        || world
            .get::<CreatureMotion>(unit)
            .is_some_and(|motion| *motion != CreatureMotion::Still)
        || world.get::<shared::casting::CastState>(unit).is_some()
    {
        return Ok(());
    }
    let position = *world
        .get::<Position>(unit)
        .ok_or("drinking follower has no position")?;
    let template = world
        .resource::<crate::item_templates::ItemTemplates>()
        .get(ItemDefinitionSource::Retail, item)
        .ok_or_else(|| format!("drink item {item} missing"))?;
    let on_use = template.on_use.ok_or("drink item has no use spell")?;
    let item_level = template.scaling.item_level;
    match crate::spell_cast::cast_item_spell(
        world,
        unit,
        on_use.spell_id,
        Some(unit),
        on_use.cooldown,
        item_level,
        Some(item),
    ) {
        Ok(()) => {
            world.entity_mut(unit).insert(DrinkingAt(position));
        }
        Err(crate::spell_cast::CastFailure::OnCooldown) => {}
        Err(error) => return Err(format!("item {item} cast failed: {error:?}")),
    }
    Ok(())
}

/// Follower movement is server-driven rather than PlayerMotion-driven.
pub(super) fn update(world: &mut World) {
    let drinking: Vec<_> = world
        .query::<(Entity, &DrinkingAt)>()
        .iter(world)
        .map(|(unit, at)| (unit, at.0))
        .collect();
    for (unit, position) in drinking {
        let interrupted = world
            .get::<Position>(unit)
            .is_none_or(|current| *current != position)
            || world
                .get::<CombatStatus>(unit)
                .is_some_and(|status| status.0)
            || world.get::<shared::casting::CastState>(unit).is_some();
        if interrupted {
            crate::stand_state::set_stand_state(world, unit, StandState::Stand);
        }
        if interrupted || !crate::auras::has_aura(world, unit, 1291791) {
            world.entity_mut(unit).remove::<DrinkingAt>();
        }
    }
}
