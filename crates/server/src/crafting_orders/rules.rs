use super::{Order, catalog::OrderCatalog, store};
use crate::item_templates::ItemTemplates;
use crate::persistence::mail_store::{MailData, MailItemData, MailSender};
use crate::persistence::{self, ClientId, GameDatabase, InventoryItemData};
use crate::profession::{PlayerProfessions, ProfessionData};
use bevy::prelude::*;
use redb::Database;
use shared::item_data::ItemDefinitionSource::Retail;

pub(crate) struct PlaceOrder {
    pub spell_id: u32,
    pub kind: u8,
    pub recipient: Option<String>,
    pub commission: u64,
    pub supplied: Vec<(u32, u32, u32)>,
    pub duration_hours: u8,
    pub note: String,
}

pub(crate) fn character(world: &World, player: Entity) -> Result<u64, String> {
    world
        .get::<ClientId>(player)
        .map(|id| id.0)
        .ok_or_else(|| "character unavailable".into())
}

pub(crate) fn eligible(db: &Database, order: &Order, character: u64) -> bool {
    match order.kind {
        0 => true,
        1 => order.guild.is_some_and(|guild| {
            persistence::load_character_guild(db, character)
                .is_some_and(|member| member.guild_id == guild)
        }),
        2 => order.recipient == Some(character),
        _ => false,
    }
}

pub(crate) fn place(
    world: &mut World,
    player: Entity,
    request: PlaceOrder,
    now: u64,
) -> Result<u64, String> {
    let customer = character(world, player)?;
    let db = &world.resource::<GameDatabase>().db;
    let recipe = world
        .resource::<OrderCatalog>()
        .0
        .get(&request.spell_id)
        .ok_or("recipe has no imported fixed-output crafting order")?;
    let templates = world.resource::<ItemTemplates>();
    if templates.get(Retail, recipe.item_id).is_none() {
        return Err("output item definition unavailable".into());
    }
    if request.duration_hours == 0 {
        return Err("order duration must be supplied explicitly".into());
    }
    if request.commission > crate::gold::MAX_MONEY {
        return Err("commission exceeds money cap".into());
    }
    let (guild, recipient) =
        load_destination(db, customer, request.kind, request.recipient.as_deref())?;
    let requirements = world
        .resource::<ProfessionData>()
        .reagents(request.spell_id);
    let contributions = validate_contributions(requirements, request.supplied)?;
    let before = persistence::load_inventory(db, customer);
    let mut model = crate::inventory::load_model(db, templates, customer)
        .map_err(|e| format!("inventory: {e:?}"))?;
    let gold = model
        .gold()
        .checked_sub(request.commission)
        .ok_or("not enough money for commission")?;
    let mut held = Vec::new();
    for &(item, _, supplied) in &contributions {
        held.extend(take_reagent(db, &mut model, item, supplied)?);
    }
    model.set_gold(gold);
    let order = Order {
        id: persistence::next_id(db, "crafting_order"),
        spell_id: recipe.spell_id,
        skill_line: recipe.skill_line,
        item_id: recipe.item_id,
        count: recipe.count,
        kind: request.kind,
        customer,
        recipient,
        guild,
        crafter: None,
        commission: request.commission,
        reagents: contributions,
        held,
        expires_at: now + u64::from(request.duration_hours) * 3600,
        note: request.note,
        status: 0,
    };
    store::commit(
        db,
        None,
        &order,
        &[(customer, before, model.into_record())],
        &[],
    )?;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record(customer, gold);
    Ok(order.id)
}

fn load_destination(
    db: &Database,
    customer: u64,
    kind: u8,
    recipient: Option<&str>,
) -> Result<(Option<u32>, Option<u64>), String> {
    let guild = match kind {
        1 => Some(
            persistence::load_character_guild(db, customer)
                .ok_or("guild order requires guild membership")?
                .guild_id,
        ),
        0 | 2 => None,
        _ => return Err("unknown order type".into()),
    };
    let recipient = match (kind, recipient) {
        (2, Some(name)) => Some(
            persistence::find_character_id_by_name_ignore_case(db, name)
                .ok_or("personal recipient not found")?,
        ),
        (2, None) => return Err("personal order requires named crafter".into()),
        (_, None) => None,
        _ => return Err("recipient only applies to personal orders".into()),
    };
    if recipient == Some(customer) {
        return Err("cannot place a personal order to yourself".into());
    }
    Ok((guild, recipient))
}

fn validate_contributions(
    requirements: &[(u32, u32)],
    mut contributions: Vec<(u32, u32, u32)>,
) -> Result<Vec<(u32, u32, u32)>, String> {
    contributions.sort_by_key(|row| row.0);
    let mut expected = requirements.to_vec();
    expected.sort_unstable();
    if contributions.len() != expected.len()
        || contributions.iter().zip(&expected).any(
            |(&(item, quantity, supplied), &(required_item, required_count))| {
                item != required_item || quantity != required_count || supplied > quantity
            },
        )
    {
        return Err("reagents must match exact authored recipe requirements".into());
    }
    Ok(contributions)
}

pub(crate) fn take_reagent(
    db: &Database,
    model: &mut crate::inventory::inventory_model::InventoryModel,
    item: u32,
    count: u32,
) -> Result<Vec<InventoryItemData>, String> {
    if model.bag_count(Retail, item) < count {
        return Err(format!("missing reagent {item}"));
    }
    let stacks: Vec<_> = model
        .bag_items()
        .filter(|stack| stack.item_id == item && model.definition_source(stack.item_guid) == Retail)
        .map(|stack| (stack.item_guid, stack.stack_count))
        .collect();
    let mut remaining = count;
    let mut taken = Vec::new();
    for (guid, available) in stacks {
        if remaining == 0 {
            break;
        }
        let count = available.min(remaining);
        taken.push(
            model
                .take_bag_item(guid, count, &mut || {
                    persistence::next_id(db, "item_instance")
                })
                .map_err(|e| format!("take reagent: {e:?}"))?,
        );
        remaining -= count;
    }
    Ok(taken)
}

pub(crate) fn browse(
    world: &World,
    player: Entity,
    skill: u32,
    kind: u8,
    mine: bool,
    now: u64,
) -> Result<Vec<Order>, String> {
    let viewer = character(world, player)?;
    let db = &world.resource::<GameDatabase>().db;
    Ok(store::all(db)?
        .into_iter()
        .filter(|order| {
            (skill == 0 || order.skill_line == skill)
                && order.kind == kind
                && if mine {
                    order.customer == viewer
                } else {
                    order.customer != viewer
                        && order.status <= 1
                        && order.expires_at > now
                        && eligible(db, order, viewer)
                }
        })
        .collect())
}

pub(crate) fn check_crafter(
    world: &World,
    player: Entity,
    order: &Order,
    now: u64,
) -> Result<u64, String> {
    let crafter = character(world, player)?;
    if order.status > 1 || order.expires_at <= now {
        return Err("order is not active".into());
    }
    if crafter == order.customer || !eligible(&world.resource::<GameDatabase>().db, order, crafter)
    {
        return Err("order is not addressed to this crafter".into());
    }
    if !world
        .get::<PlayerProfessions>(player)
        .is_some_and(|p| p.knows(order.spell_id))
    {
        return Err("crafter does not know recipe".into());
    }
    Ok(crafter)
}

pub(crate) fn claim(world: &mut World, player: Entity, id: u64, now: u64) -> Result<(), String> {
    let db = &world.resource::<GameDatabase>().db;
    let before = store::load(db, id)?;
    let crafter = check_crafter(world, player, &before, now)?;
    if before.status != 0 {
        return Err("order already claimed".into());
    }
    let mut order = before.clone();
    order.crafter = Some(crafter);
    order.status = 1;
    store::commit(db, Some(&before), &order, &[], &[])
}

pub(crate) fn release(world: &mut World, player: Entity, id: u64) -> Result<(), String> {
    if super::cast::is_order_cast(
        world,
        player,
        store::load(&world.resource::<GameDatabase>().db, id)?.spell_id,
    ) {
        return Err("cannot release while crafting".into());
    }
    let db = &world.resource::<GameDatabase>().db;
    let before = store::load(db, id)?;
    if before.status != 1 || before.crafter != Some(character(world, player)?) {
        return Err("only assigned crafter can release".into());
    }
    let mut order = before.clone();
    order.crafter = None;
    order.status = 0;
    store::commit(db, Some(&before), &order, &[], &[])
}

pub(crate) fn cancel(world: &mut World, player: Entity, id: u64, now: u64) -> Result<(), String> {
    let db = &world.resource::<GameDatabase>().db;
    let order = store::load(db, id)?;
    if order.customer != character(world, player)? || order.status != 0 {
        return Err("only customer can cancel an unclaimed order".into());
    }
    refund(db, &order, now)
}

pub(crate) fn expire(db: &Database, now: u64) -> Result<(), String> {
    for order in store::all(db)?
        .into_iter()
        .filter(|o| o.status <= 1 && o.expires_at <= now)
    {
        refund(db, &order, now)?;
    }
    Ok(())
}

fn refund(db: &Database, before: &Order, now: u64) -> Result<(), String> {
    let mails = delivery(
        db,
        before.customer,
        "Crafting order refunded",
        &before.held,
        before.commission,
        now,
    );
    let mut order = before.clone();
    order.status = 3;
    order.held.clear();
    store::commit(db, Some(before), &order, &[], &mails)
}

/// Order attachments use normal server mail retention, not guessed order durations.
pub(crate) fn delivery(
    db: &Database,
    recipient: u64,
    subject: &str,
    items: &[InventoryItemData],
    money: u64,
    now: u64,
) -> Vec<(u64, MailData)> {
    let chunks = items
        .len()
        .max(1)
        .div_ceil(shared::protocol::MAX_MAIL_ATTACHMENTS);
    (0..chunks)
        .map(|chunk| {
            let start = chunk * shared::protocol::MAX_MAIL_ATTACHMENTS;
            let end = (start + shared::protocol::MAX_MAIL_ATTACHMENTS).min(items.len());
            (
                recipient,
                MailData {
                    mail_id: persistence::next_id(db, "mail"),
                    sender: MailSender::CraftingOrder,
                    subject: subject.into(),
                    body: String::new(),
                    items: items[start.min(items.len())..end]
                        .iter()
                        .enumerate()
                        .map(|(slot, item)| MailItemData {
                            slot: slot as u8,
                            item: item.clone(),
                        })
                        .collect(),
                    money: if chunk == 0 { money } else { 0 },
                    cod: 0,
                    delivers_at: now,
                    expires_at: now + 30 * 86400,
                    read: false,
                    returned: false,
                    cod_payment: false,
                },
            )
        })
        .collect()
}
