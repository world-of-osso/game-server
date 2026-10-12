use super::{Order, rules};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    CraftingOrderKind, CraftingOrderReagent, CraftingOrderRequest, CraftingOrderSnapshot,
    CraftingOrderStateUpdate, CraftingOrderStatus, CraftingOrdersSnapshot, ProfessionChannel,
};

#[derive(Component, Clone, Copy)]
struct View {
    skill: u32,
    kind: CraftingOrderKind,
    mine: bool,
}
#[derive(Resource, Default)]
struct NextSweep(u64);

pub(crate) struct CraftingOrdersPlugin;
impl Plugin for CraftingOrdersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::catalog::OrderCatalog>()
            .init_resource::<NextSweep>()
            .add_systems(Update, (sweep, receive).chain());
    }
}

fn code(kind: CraftingOrderKind) -> u8 {
    match kind {
        CraftingOrderKind::Public => 0,
        CraftingOrderKind::Guild => 1,
        CraftingOrderKind::Personal => 2,
    }
}

fn sweep(world: &mut World) {
    let now = crate::mail::unix_now();
    if world.resource::<NextSweep>().0 > now {
        return;
    }
    world.resource_mut::<NextSweep>().0 = now + 1;
    let Some(db) = world.get_resource::<GameDatabase>() else {
        return;
    };
    let result = super::all(&db.db).and_then(|orders| {
        let customers: Vec<_> = orders
            .iter()
            .filter(|o| o.status <= 1 && o.expires_at <= now)
            .map(|o| o.customer)
            .collect();
        rules::expire(&db.db, now)?;
        Ok(customers)
    });
    match result {
        Ok(customers) => {
            for customer in customers {
                mail_changed(world, customer);
            }
        }
        Err(error) => error!("Crafting order expiry failed: {error}"),
    }
}

fn receive(world: &mut World) {
    let mut query = world.query_filtered::<(
        &PlayerLink,
        Has<InWorld>,
        &mut MessageReceiver<CraftingOrderRequest>,
    ), With<Authenticated>>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .filter(|(_, inside, _)| *inside)
        .flat_map(|(link, _, mut receiver)| {
            receiver
                .receive()
                .map(|request| (link.0, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (player, request) in requests {
        let result = handle(world, player, request);
        send_result(world, player, result);
    }
}

fn handle(
    world: &mut World,
    player: Entity,
    request: CraftingOrderRequest,
) -> Result<String, String> {
    let now = crate::mail::unix_now();
    match request {
        CraftingOrderRequest::Query {
            skill_line,
            kind,
            mine,
        } => {
            world.entity_mut(player).insert(View {
                skill: skill_line,
                kind,
                mine,
            });
            Ok("Orders refreshed".into())
        }
        CraftingOrderRequest::Place {
            spell_id,
            kind,
            recipient,
            commission,
            reagents,
            duration_hours,
            note,
        } => {
            require_customer_service(world, player)?;
            let skill = world
                .resource::<super::catalog::OrderCatalog>()
                .0
                .get(&spell_id)
                .ok_or("recipe not in fixed-output order catalog")?
                .skill_line;
            rules::place(
                world,
                player,
                rules::PlaceOrder {
                    spell_id,
                    kind: code(kind),
                    recipient,
                    commission,
                    supplied: reagents
                        .into_iter()
                        .map(|r| (r.item_id, r.quantity, r.supplied))
                        .collect(),
                    duration_hours,
                    note,
                },
                now,
            )?;
            world.entity_mut(player).insert(View {
                skill,
                kind,
                mine: true,
            });
            Ok("Order placed".into())
        }
        CraftingOrderRequest::Claim { order_id } => {
            rules::claim(world, player, order_id, now)?;
            Ok("Order claimed".into())
        }
        CraftingOrderRequest::Release { order_id } => {
            rules::release(world, player, order_id)?;
            Ok("Order released".into())
        }
        CraftingOrderRequest::Fulfill { order_id } => {
            super::begin_fulfill(world, player, order_id, now)?;
            Ok("Craft started".into())
        }
        CraftingOrderRequest::Cancel { order_id } => {
            require_customer_service(world, player)?;
            rules::cancel(world, player, order_id, now)?;
            mail_changed(world, rules::character(world, player)?);
            Ok("Order refunded".into())
        }
    }
}

fn require_customer_service(world: &mut World, player: Entity) -> Result<(), String> {
    let mut sessions = world.query::<(&PlayerLink, &crate::npc_interaction::ActiveInteraction)>();
    let broker = sessions
        .iter(world)
        .find(|(link, active)| {
            link.0 == player && active.role == Some(shared::protocol::NpcRole::CraftingOrders)
        })
        .map(|(_, active)| active.npc)
        .ok_or("Open the crafting order broker before changing customer orders")?;
    crate::npc_interaction::find_interactable_crafting_broker(world, player, broker)
        .map_err(|error| error.message().to_string())?;
    Ok(())
}

pub(crate) fn mail_changed(world: &mut World, id: u64) {
    if let Some(mut changed) = world.get_resource_mut::<crate::mail::ChangedMailboxes>() {
        changed.0.insert(id);
    }
}

pub(crate) fn send_result(world: &mut World, player: Entity, result: Result<String, String>) {
    let mut reply = CraftingOrderStateUpdate {
        snapshot: None,
        message: result.as_ref().ok().cloned(),
        error: result.err(),
    };
    if let Some(view) = world.get::<View>(player).copied() {
        match load_view_snapshot(world, player, view) {
            Ok(snapshot) => reply.snapshot = Some(snapshot),
            Err(error) => reply.error = Some(error),
        }
    }
    let mut connections =
        world.query::<(&PlayerLink, &mut MessageSender<CraftingOrderStateUpdate>)>();
    if let Some((_, mut sender)) = connections
        .iter_mut(world)
        .find(|(link, _)| link.0 == player)
    {
        sender.send::<ProfessionChannel>(reply);
    }
}

fn load_view_snapshot(
    world: &World,
    player: Entity,
    view: View,
) -> Result<CraftingOrdersSnapshot, String> {
    let orders = rules::browse(
        world,
        player,
        view.skill,
        code(view.kind),
        view.mine,
        crate::mail::unix_now(),
    )?;
    let orders = orders
        .iter()
        .map(|order| snapshot(world, player, order))
        .collect::<Result<Vec<_>, _>>()?;
    let data = world.resource::<crate::profession::ProfessionData>();
    let mut recipes: Vec<_> = world
        .resource::<super::catalog::OrderCatalog>()
        .0
        .values()
        .filter(|recipe| view.skill == 0 || recipe.skill_line == view.skill)
        .map(|recipe| shared::protocol::CraftingOrderRecipeSnapshot {
            spell_id: recipe.spell_id,
            skill_line: recipe.skill_line,
            item_id: recipe.item_id,
            count: recipe.count,
            name: recipe.name.clone(),
            reagents: data
                .reagents(recipe.spell_id)
                .iter()
                .map(|&(item_id, quantity)| CraftingOrderReagent {
                    item_id,
                    quantity,
                    supplied: 0,
                })
                .collect(),
        })
        .collect();
    recipes.sort_by_key(|recipe| recipe.spell_id);
    Ok(CraftingOrdersSnapshot {
        skill_line: view.skill,
        kind: view.kind,
        mine: view.mine,
        recipes,
        orders,
    })
}

fn snapshot(world: &World, player: Entity, order: &Order) -> Result<CraftingOrderSnapshot, String> {
    let db = &world.resource::<GameDatabase>().db;
    let name = |id| {
        persistence::load_character(db, id)
            .map(|c| c.name)
            .ok_or_else(|| format!("order character {id} is missing"))
    };
    let viewer = rules::character(world, player)?;
    let can_craft = rules::check_crafter(world, player, order, crate::mail::unix_now()).is_ok();
    let assigned = can_craft && order.status == 1 && order.crafter == Some(viewer);
    let can_fulfill = if assigned {
        let model = crate::inventory::load_model(
            db,
            world.resource::<crate::item_templates::ItemTemplates>(),
            viewer,
        )
        .map_err(|error| format!("load order crafter inventory: {error:?}"))?;
        order.reagents.iter().all(|&(item, required, supplied)| {
            model.bag_count(shared::item_data::ItemDefinitionSource::Retail, item)
                >= required - supplied
        })
    } else {
        false
    };
    Ok(CraftingOrderSnapshot {
        id: order.id,
        spell_id: order.spell_id,
        item_id: order.item_id,
        count: order.count,
        kind: match order.kind {
            0 => CraftingOrderKind::Public,
            1 => CraftingOrderKind::Guild,
            2 => CraftingOrderKind::Personal,
            _ => unreachable!("stored order kind"),
        },
        customer: name(order.customer)?,
        recipient: order.recipient.map(name).transpose()?,
        crafter: order.crafter.map(name).transpose()?,
        commission: order.commission,
        reagents: order
            .reagents
            .iter()
            .map(|&(item_id, quantity, supplied)| CraftingOrderReagent {
                item_id,
                quantity,
                supplied,
            })
            .collect(),
        expires_at: order.expires_at,
        note: order.note.clone(),
        status: match order.status {
            0 => CraftingOrderStatus::Open,
            1 => CraftingOrderStatus::Claimed,
            2 => CraftingOrderStatus::Completed,
            3 => CraftingOrderStatus::Cancelled,
            _ => unreachable!("stored order status"),
        },
        can_claim: can_craft && order.status == 0,
        can_fulfill,
        can_cancel: order.status == 0 && order.customer == viewer,
        can_release: order.status == 1
            && order.crafter == Some(viewer)
            && !super::is_order_cast(world, player, order.spell_id),
    })
}
