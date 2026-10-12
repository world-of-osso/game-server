use super::{rules, store};
use crate::persistence::{self, GameDatabase};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
struct OrderCast {
    id: u64,
    spell_id: u32,
}

pub(crate) fn is_order_cast(world: &World, player: Entity, spell_id: u32) -> bool {
    world
        .get::<OrderCast>(player)
        .is_some_and(|cast| cast.spell_id == spell_id)
}

pub(crate) fn interrupt(world: &mut World, player: Entity) {
    world.entity_mut(player).remove::<OrderCast>();
}

pub(crate) fn begin_fulfill(
    world: &mut World,
    player: Entity,
    id: u64,
    now: u64,
) -> Result<(), String> {
    if world.get::<shared::casting::CastState>(player).is_some()
        || world.get::<OrderCast>(player).is_some()
    {
        return Err("already casting".into());
    }
    let order = store::load(&world.resource::<GameDatabase>().db, id)?;
    let crafter = rules::check_crafter(world, player, &order, now)?;
    if order.status != 1 || order.crafter != Some(crafter) {
        return Err("only assigned crafter can fulfill".into());
    }
    world.entity_mut(player).insert(OrderCast {
        id,
        spell_id: order.spell_id,
    });
    if let Err(failure) = crate::spell_cast::cast_for_player(world, player, order.spell_id) {
        interrupt(world, player);
        return Err(format!("craft cast rejected: {failure:?}"));
    }
    Ok(())
}

pub(crate) fn cast_requirements(
    world: &World,
    player: Entity,
    spell: &SpellInfo,
) -> Option<Result<Vec<(u32, u32)>, String>> {
    let cast = world.get::<OrderCast>(player)?;
    if cast.spell_id != spell.id {
        return None;
    }
    Some((|| {
        let order = store::load(&world.resource::<GameDatabase>().db, cast.id)?;
        let crafter = rules::check_crafter(world, player, &order, crate::mail::unix_now())?;
        if order.status != 1 || order.crafter != Some(crafter) {
            return Err("order is no longer assigned".into());
        }
        Ok(order
            .reagents
            .iter()
            .map(|&(item, required, supplied)| (item, required - supplied))
            .collect())
    })())
}

pub(crate) fn complete_effect(
    world: &mut World,
    player: Entity,
    spell_id: u32,
    item_id: u32,
    count: u32,
) -> Option<Result<(), String>> {
    let cast = *world.get::<OrderCast>(player)?;
    if cast.spell_id != spell_id {
        return None;
    }
    let result = complete(world, player, cast, item_id, count);
    interrupt(world, player);
    super::runtime::send_result(
        world,
        player,
        result
            .as_ref()
            .map(|_| "Order completed".to_owned())
            .map_err(Clone::clone),
    );
    Some(result)
}

fn complete(
    world: &mut World,
    player: Entity,
    cast: OrderCast,
    item_id: u32,
    count: u32,
) -> Result<(), String> {
    let now = crate::mail::unix_now();
    let db = &world.resource::<GameDatabase>().db;
    let before = store::load(db, cast.id)?;
    let crafter = rules::check_crafter(world, player, &before, now)?;
    if before.status != 1
        || before.crafter != Some(crafter)
        || before.item_id != item_id
        || before.count != count
    {
        return Err("order output or assignment changed".into());
    }
    let output = crate::profession::new_order_item(world, player, item_id, count)?;
    let templates = world.resource::<crate::item_templates::ItemTemplates>();
    let inventory = persistence::load_inventory(db, crafter);
    let mut model = crate::inventory::load_model(db, templates, crafter)
        .map_err(|e| format!("inventory: {e:?}"))?;
    for &(item, required, supplied) in &before.reagents {
        rules::take_reagent(db, &mut model, item, required - supplied)?;
    }
    let mut mails = rules::delivery(
        db,
        before.customer,
        "Crafting order complete",
        &[output],
        0,
        now,
    );
    mails.extend(rules::delivery(
        db,
        crafter,
        "Crafting order commission",
        &[],
        before.commission,
        now,
    ));
    let mut order = before.clone();
    order.status = 2;
    order.held.clear();
    store::commit(
        db,
        Some(&before),
        &order,
        &[(crafter, inventory, model.into_record())],
        &mails,
    )?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record_items(crafter);
    crate::achievement::record_order_fulfillment(world, player, before.kind);
    super::runtime::mail_changed(world, before.customer);
    super::runtime::mail_changed(world, crafter);
    Ok(())
}
