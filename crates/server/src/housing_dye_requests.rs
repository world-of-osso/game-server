//! Transient preview belongs to one live connection/player, never persisted admission.
use super::*;
use shared::protocol::{DecorDye, DecorDyePreview};

#[derive(Component, Clone)]
struct DyePreview {
    player: Entity,
    selection: DecorDyePreview,
}

pub(super) fn preview(
    world: &mut World,
    actor: Actor,
    context: store::Actor,
    house_id: u64,
    instance_id: u64,
    changes: &[DecorDye],
) -> Result<(), String> {
    let selection = store::preview_dyes(
        &world.resource::<GameDatabase>().db,
        context,
        house_id,
        instance_id,
        changes,
        &catalog(world)?.dyes,
    )?;
    world.entity_mut(actor.connection).insert(DyePreview {
        player: actor.player,
        selection,
    });
    Ok(())
}

pub(super) fn commit_preview(
    world: &mut World,
    actor: Actor,
    context: store::Actor,
    house_id: u64,
    instance_id: u64,
) -> Result<(), String> {
    let staged = world
        .get::<DyePreview>(actor.connection)
        .filter(|preview| {
            preview.player == actor.player
                && preview.selection.house_id == house_id
                && preview.selection.instance_id == instance_id
        })
        .ok_or("No active dye preview for this current player and decor.")?
        .selection
        .clone();
    let changed = store::commit_dyes(
        &world.resource::<GameDatabase>().db,
        context,
        &staged,
        &catalog(world)?.dyes,
    )?;
    world.entity_mut(actor.connection).remove::<DyePreview>();
    if changed {
        world
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(actor.character);
    }
    Ok(())
}

pub(super) fn cancel(world: &mut World, actor: Actor) {
    world.entity_mut(actor.connection).remove::<DyePreview>();
}

pub(super) fn attach_preview(world: &World, actor: Actor, state: &mut HousingState) {
    let Some(preview) = world
        .get::<DyePreview>(actor.connection)
        .filter(|preview| preview.player == actor.player)
    else {
        return;
    };
    let target = state
        .layout
        .as_ref()
        .filter(|layout| layout.house_id == preview.selection.house_id && layout.can_edit)
        .and_then(|layout| {
            layout
                .decor
                .iter()
                .find(|decor| decor.instance_id == preview.selection.instance_id)
        });
    if target.is_some() {
        state.dye_preview = Some(preview.selection.clone());
    }
}
