//! Player mail (docs/wiki/systems/mail.md) at the Mailbox whose frame
//! `npc_interaction` has open.
//!
//! Rules follow AzerothCore `MailHandler.cpp` / `Mail.cpp`: postage 30c per item
//! (30c for a letter), money and C.O.D. exclusive and C.O.D. dropped without items,
//! no mail to yourself or to the other faction (`AllowTwoSide.Interaction.Mail` 0),
//! soulbound items refused, 100 mails per recipient, items to another account's
//! character delivered after an hour, C.O.D. mail expiring after 3 days and other
//! mail after 30, C.O.D. paid on the first item taken and mailed to the sender,
//! taking money refused at the gold cap, returning swapping sender and recipient,
//! and expired mail returned when it is returnable and still holds something,
//! otherwise deleted. Every change reaches the character's open mailbox, and
//! `PendingMail` follows the unread delivered mail.

use std::collections::HashSet;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::protocol::{
    ItemStack, MAIL_BODY_MAX_LETTERS, MAIL_SUBJECT_MAX_LETTERS, MAX_MAIL_ATTACHMENTS, MailAction,
    MailAttachment, MailChannel, MailError, MailFailed, MailHeader, MailRequest, MailSent,
    MailboxContents, NpcRole, PendingMail, SendMail,
};

use crate::auth::InWorld;
use crate::faction::faction_for_race;
use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::inventory::{inventory_model::InventoryModel, load_model};
use crate::item_templates::ItemTemplates;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::inventory_store::write_inventory_record;
use crate::persistence::mail_store::{
    self, MailData, MailItemData, MailSender, load_mailbox, push_mail, write_mailbox,
};
use crate::persistence::{self, GameDatabase};

/// TrinityCore `HandleSendMail`: a mailbox holding more than this takes no more mail.
const MAX_MAILS_PER_RECIPIENT: usize = 100;
/// How often delivery, expiry and `PendingMail` are checked.
const MAIL_TICK: Duration = Duration::from_secs(1);

pub(crate) fn unix_now() -> u64 {
    crate::guild_bank::unix_now()
}

pub struct MailPlugin;

impl Plugin for MailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChangedMailboxes>().add_systems(
            Update,
            (
                send_opened_mailboxes,
                process_send_requests,
                process_mail_requests,
                tick_mail.run_if(mail_tick_due),
                send_changed_mailboxes,
            )
                .chain(),
        );
    }
}

fn mail_tick_due(time: Res<Time>, mut elapsed: Local<Option<Duration>>) -> bool {
    let Some(since) = elapsed.as_mut() else {
        *elapsed = Some(Duration::ZERO);
        return true;
    };
    *since += time.delta();
    if *since < MAIL_TICK {
        return false;
    }
    *since = Duration::ZERO;
    true
}

/// Characters whose mailbox changed; their open mailboxes get the inbox again.
#[derive(Resource, Default)]
pub(crate) struct ChangedMailboxes(pub(crate) HashSet<u64>);

/// What a mail operation changed: bags and gold to replicate, mailboxes to resend.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct MailChanges {
    pub gold: Vec<(u64, u64)>,
    pub mailboxes: Vec<u64>,
}

/// `PendingMail` last sent on this connection, with the delivered mail count it saw.
#[derive(Component, Debug, Default)]
struct SentPendingMail {
    senders: Option<Vec<String>>,
    delivered: usize,
}

type MailResult = Result<MailChanges, MailError>;

// --- Rules ---

/// The inbox a mailbox shows: delivered mail, newest first.
pub(crate) fn inbox(mails: &[MailData], now: u64) -> Vec<&MailData> {
    let mut delivered: Vec<&MailData> = mails.iter().filter(|m| m.is_delivered(now)).collect();
    delivered.sort_by_key(|mail| std::cmp::Reverse((mail.delivers_at, mail.mail_id)));
    delivered
}

/// Retail `GetLatestThreeSenders`: senders of the newest unread delivered mail.
pub(crate) fn pending_senders(mails: &[MailData], now: u64) -> Vec<String> {
    let mut senders: Vec<String> = Vec::new();
    for mail in inbox(mails, now).into_iter().filter(|mail| !mail.read) {
        let name = mail.sender.name();
        if !senders.iter().any(|sender| sender == name) {
            senders.push(name.to_owned());
        }
        if senders.len() == 3 {
            break;
        }
    }
    senders
}

fn header(
    db: &Database,
    templates: &ItemTemplates,
    mail: &MailData,
    soulbound: &HashSet<u64>,
) -> MailHeader {
    MailHeader {
        mail_id: mail.mail_id,
        sender: mail.sender.name().to_owned(),
        subject: mail.subject.clone(),
        body: mail.body.clone(),
        money: mail.money,
        cod: mail.cod,
        attachments: mail
            .items
            .iter()
            .map(|attached| MailAttachment {
                slot: attached.slot,
                item: ItemStack {
                    item_guid: attached.item.item_guid,
                    item_id: attached.item.item_id,
                    definition_source: persistence::inventory_store::load_item_definition_source(
                        db,
                        attached.item.item_guid,
                    ),
                    count: attached.item.stack_count,
                    durability: None,
                    soulbound: soulbound.contains(&attached.item.item_guid),
                    crafting_origin: crate::inventory::stored_crafting_origin(
                        db,
                        templates,
                        &attached.item,
                    ),
                },
                name: attached.item.name.clone(),
                quality: attached.item.quality,
            })
            .collect(),
        expires_at: mail.expires_at,
        read: mail.read,
        returned: mail.returned,
        from_player: matches!(mail.sender, MailSender::Player { .. }),
        returnable: mail.returnable(),
    }
}

pub(crate) fn mailbox_contents(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    object: u64,
) -> MailboxContents {
    let now = unix_now();
    let mails = load_mailbox(db, character_id);
    let guids = mails
        .iter()
        .flat_map(|mail| mail.items.iter().map(|attached| attached.item.item_guid));
    let soulbound = persistence::inventory_store::soulbound_guids(db, guids);
    MailboxContents {
        object,
        mails: inbox(&mails, now)
            .into_iter()
            .map(|mail| header(db, templates, mail, &soulbound))
            .collect(),
        now,
    }
}

fn truncate_letters(text: &str, letters: usize) -> String {
    text.chars().take(letters).collect()
}

/// Character id of `name`, compared case-insensitively like Retail name entry.
fn find_character(db: &Database, name: &str) -> Option<u64> {
    let name = name.trim();
    persistence::find_character_id_by_name_ignore_case(db, name)
}

fn load(db: &Database, templates: &ItemTemplates, id: u64) -> Result<InventoryModel, MailError> {
    load_model(db, templates, id).map_err(|_| MailError::Internal)
}

/// Remove the attachments from the sender's bags, in slot order.
fn take_attachments(
    db: &Database,
    model: &mut InventoryModel,
    guids: &[u64],
) -> Result<Vec<MailItemData>, MailError> {
    let unique: HashSet<u64> = guids.iter().copied().collect();
    if unique.len() != guids.len() {
        return Err(MailError::InvalidAttachment);
    }
    let mut mint = || persistence::next_id(db, "item_instance");
    guids
        .iter()
        .enumerate()
        .map(|(slot, &guid)| {
            model.bag_item(guid).ok_or(MailError::InvalidAttachment)?;
            if model.is_soulbound(guid) {
                return Err(MailError::BoundItem);
            }
            let item = model
                .take_bag_item(guid, 0, &mut mint)
                .map_err(|_| MailError::InvalidAttachment)?;
            Ok(MailItemData {
                slot: slot as u8,
                item,
            })
        })
        .collect()
}

/// AzerothCore `WorldSession::HandleSendMail`.
pub(crate) fn send_mail(
    db: &Database,
    templates: &ItemTemplates,
    sender_id: u64,
    request: &SendMail,
    now: u64,
) -> MailResult {
    let sender = persistence::load_character(db, sender_id).ok_or(MailError::Internal)?;
    let recipient_id =
        find_character(db, &request.recipient).ok_or(MailError::RecipientNotFound)?;
    if recipient_id == sender_id {
        return Err(MailError::ToSelf);
    }
    let recipient =
        persistence::load_character(db, recipient_id).ok_or(MailError::RecipientNotFound)?;
    if faction_for_race(sender.race) != faction_for_race(recipient.race) {
        return Err(MailError::WrongFaction);
    }
    if request.money > 0 && request.cod > 0 {
        return Err(MailError::Internal);
    }
    if request.attachments.len() > MAX_MAIL_ATTACHMENTS {
        return Err(MailError::TooManyAttachments);
    }
    if load_mailbox(db, recipient_id).len() > MAX_MAILS_PER_RECIPIENT {
        return Err(MailError::RecipientMailboxFull);
    }
    let mut model = load(db, templates, sender_id)?;
    let cost = shared::mail::postage(request.attachments.len())
        .checked_add(request.money)
        .ok_or(MailError::NotEnoughMoney)?;
    if model.gold() < cost {
        return Err(MailError::NotEnoughMoney);
    }
    let items = take_attachments(db, &mut model, &request.attachments)?;
    model.set_gold(model.gold() - cost);
    // TrinityCore `HandleSendMail`: mail between guild members arrives instantly.
    let instant =
        same_account(db, sender_id, recipient_id) || same_guild(db, sender_id, recipient_id);
    let delivers_at = now + shared::mail::delivery_delay(!items.is_empty(), instant);
    let cod = if items.is_empty() { 0 } else { request.cod };
    let mail = MailData {
        mail_id: persistence::next_id(db, "mail"),
        sender: MailSender::Player {
            character_id: sender_id,
            name: sender.name,
        },
        subject: truncate_letters(&request.subject, MAIL_SUBJECT_MAX_LETTERS),
        body: truncate_letters(&request.body, MAIL_BODY_MAX_LETTERS),
        items,
        money: request.money,
        cod,
        delivers_at,
        expires_at: delivers_at + shared::mail::expiry_secs(cod),
        read: false,
        returned: false,
        cod_payment: false,
    };
    let gold = model.gold();
    let txn = db.begin_write().expect("Failed to begin mail send");
    write_inventory_record(&txn, sender_id, &model.into_record());
    push_mail(&txn, recipient_id, mail);
    txn.commit().expect("Failed to commit mail send");
    Ok(MailChanges {
        gold: vec![(sender_id, gold)],
        mailboxes: vec![sender_id, recipient_id],
    })
}

fn same_guild(db: &Database, a: u64, b: u64) -> bool {
    let guild = |id| persistence::load_character_guild(db, id).map(|g| g.guild_id);
    guild(a).is_some_and(|id| guild(b) == Some(id))
}

fn same_account(db: &Database, a: u64, b: u64) -> bool {
    persistence::find_account_id_for_character(db, a)
        == persistence::find_account_id_for_character(db, b)
}

/// A new mail from `mail`'s recipient back to `to`, its sender (TrinityCore
/// `MailDraft::SendReturnToSender` with `MAIL_CHECK_MASK_RETURNED`): items going to
/// another account's character wait the delivery delay.
fn returned_mail(db: &Database, mail: MailData, from: u64, to: u64, now: u64) -> MailData {
    let name = persistence::load_character(db, from).map_or_else(String::new, |c| c.name);
    let delivers_at =
        now + shared::mail::delivery_delay(!mail.items.is_empty(), same_account(db, from, to));
    MailData {
        mail_id: persistence::next_id(db, "mail"),
        sender: MailSender::Player {
            character_id: from,
            name,
        },
        subject: mail.subject,
        body: mail.body,
        items: mail.items,
        money: mail.money,
        cod: 0,
        delivers_at,
        expires_at: delivers_at + shared::mail::MAIL_EXPIRY_SECS,
        read: false,
        returned: true,
        cod_payment: false,
    }
}

/// One request against one delivered mail of `character_id`.
pub(crate) fn act_on_mail(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    mail_id: u64,
    action: MailAction,
    now: u64,
) -> MailResult {
    let mut mails = load_mailbox(db, character_id);
    let index = mails
        .iter()
        .position(|mail| mail.mail_id == mail_id && mail.is_delivered(now))
        .ok_or(MailError::Internal)?;
    match action {
        MailAction::MarkRead => {
            mails[index].read = true;
            let txn = db.begin_write().expect("Failed to begin mail write");
            write_mailbox(&txn, character_id, &mails);
            txn.commit().expect("Failed to commit mail write");
            Ok(MailChanges {
                gold: Vec::new(),
                mailboxes: vec![character_id],
            })
        }
        MailAction::TakeMoney => take_money(db, templates, character_id, mails, index),
        MailAction::TakeAttachment { slot } => {
            take_attachment(db, templates, character_id, mails, index, slot, now)
        }
        MailAction::Return => return_mail(db, character_id, mails, index, now),
        MailAction::Delete => {
            if mails[index].returnable() && !mails[index].is_empty() {
                return Err(MailError::CannotDelete);
            }
            mails.remove(index);
            let txn = db.begin_write().expect("Failed to begin mail delete");
            write_mailbox(&txn, character_id, &mails);
            txn.commit().expect("Failed to commit mail delete");
            Ok(MailChanges {
                gold: Vec::new(),
                mailboxes: vec![character_id],
            })
        }
    }
}

/// AzerothCore `HandleMailTakeMoney`.
fn take_money(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    mut mails: Vec<MailData>,
    index: usize,
) -> MailResult {
    let mut model = load(db, templates, character_id)?;
    let gold = model
        .gold()
        .checked_add(mails[index].money)
        .filter(|gold| *gold <= MAX_MONEY)
        .ok_or(MailError::TooMuchGold)?;
    model.set_gold(gold);
    mails[index].money = 0;
    mails[index].read = true;
    let txn = db.begin_write().expect("Failed to begin mail take money");
    write_inventory_record(&txn, character_id, &model.into_record());
    write_mailbox(&txn, character_id, &mails);
    txn.commit().expect("Failed to commit mail take money");
    Ok(MailChanges {
        gold: vec![(character_id, gold)],
        mailboxes: vec![character_id],
    })
}

/// AzerothCore `HandleMailTakeItem`: the C.O.D. is paid with the first item and
/// mailed to the sender as a C.O.D. payment.
fn take_attachment(
    db: &Database,
    templates: &ItemTemplates,
    character_id: u64,
    mut mails: Vec<MailData>,
    index: usize,
    slot: u8,
    now: u64,
) -> MailResult {
    let position = mails[index]
        .items
        .iter()
        .position(|attached| attached.slot == slot)
        .ok_or(MailError::Internal)?;
    let mut model = load(db, templates, character_id)?;
    let cod = mails[index].cod;
    if model.gold() < cod {
        return Err(MailError::CodInsufficientMoney);
    }
    let item = mails[index].items[position].item.clone();
    let source = persistence::inventory_store::load_item_definition_source(db, item.item_guid);
    let template = templates
        .get(source, item.item_id)
        .copied()
        .ok_or(MailError::Internal)?;
    let mut mint = || persistence::next_id(db, "item_instance");
    model
        .store_item(item, source, template, &mut mint)
        .map_err(|_| MailError::InventoryFull)?;
    model.set_gold(model.gold() - cod);
    let mail = &mut mails[index];
    mail.items.remove(position);
    mail.cod = 0;
    mail.read = true;
    let payment = match (&mail.sender, cod) {
        (
            MailSender::Player {
                character_id: to, ..
            },
            1..,
        ) => {
            let name =
                persistence::load_character(db, character_id).map_or_else(String::new, |c| c.name);
            Some((
                *to,
                MailData {
                    mail_id: persistence::next_id(db, "mail"),
                    sender: MailSender::Player { character_id, name },
                    subject: mail.subject.clone(),
                    body: String::new(),
                    items: Vec::new(),
                    money: cod,
                    cod: 0,
                    delivers_at: now,
                    expires_at: now + shared::mail::MAIL_EXPIRY_SECS,
                    read: false,
                    returned: false,
                    cod_payment: true,
                },
            ))
        }
        _ => None,
    };
    let gold = model.gold();
    let mut mailboxes = vec![character_id];
    let txn = db.begin_write().expect("Failed to begin mail take item");
    write_inventory_record(&txn, character_id, &model.into_record());
    write_mailbox(&txn, character_id, &mails);
    if let Some((to, payment)) = payment {
        push_mail(&txn, to, payment);
        mailboxes.push(to);
    }
    txn.commit().expect("Failed to commit mail take item");
    Ok(MailChanges {
        gold: vec![(character_id, gold)],
        mailboxes,
    })
}

/// AzerothCore `HandleMailReturnToSender`.
fn return_mail(
    db: &Database,
    character_id: u64,
    mut mails: Vec<MailData>,
    index: usize,
    now: u64,
) -> MailResult {
    if !mails[index].returnable() {
        return Err(MailError::Internal);
    }
    let MailSender::Player {
        character_id: to, ..
    } = mails[index].sender
    else {
        return Err(MailError::Internal);
    };
    let mail = mails.remove(index);
    let back = returned_mail(db, mail, character_id, to, now);
    let txn = db.begin_write().expect("Failed to begin mail return");
    write_mailbox(&txn, character_id, &mails);
    push_mail(&txn, to, back);
    txn.commit().expect("Failed to commit mail return");
    Ok(MailChanges {
        gold: Vec::new(),
        mailboxes: vec![character_id, to],
    })
}

/// AzerothCore `ObjectMgr::ReturnOrDeleteOldMails`: expired mail goes back to its
/// sender when returnable and not empty, otherwise it is deleted with its items.
pub(crate) fn expire_mail(db: &Database, now: u64) -> Vec<u64> {
    let mut changed = Vec::new();
    for (character_id, mails) in mail_store::all_mailboxes(db) {
        let (expired, kept): (Vec<MailData>, Vec<MailData>) = mails
            .into_iter()
            .partition(|mail| mail.is_delivered(now) && mail.expires_at <= now);
        if expired.is_empty() {
            continue;
        }
        let returns: Vec<(u64, MailData)> = expired
            .into_iter()
            .filter_map(|mail| match mail.sender {
                MailSender::Player {
                    character_id: to, ..
                } if mail.returnable() && !mail.is_empty() => {
                    Some((to, returned_mail(db, mail, character_id, to, now)))
                }
                _ => None,
            })
            .collect();
        let txn = db.begin_write().expect("Failed to begin mail expiry");
        write_mailbox(&txn, character_id, &kept);
        for (to, mail) in returns {
            push_mail(&txn, to, mail);
            changed.push(to);
        }
        txn.commit().expect("Failed to commit mail expiry");
        changed.push(character_id);
    }
    changed
}

/// `game-server-admin mail-deliver-now`: deliver every mail still in transit to
/// `character_id` at `now`, with its expiry counted from then. Returns the count.
pub(crate) fn deliver_pending_mail(db: &Database, character_id: u64, now: u64) -> usize {
    let mut mails = load_mailbox(db, character_id);
    let mut delivered = 0;
    for mail in mails.iter_mut().filter(|mail| !mail.is_delivered(now)) {
        mail.delivers_at = now;
        mail.expires_at = now + shared::mail::expiry_secs(mail.cod);
        delivered += 1;
    }
    if delivered > 0 {
        let txn = db.begin_write().expect("Failed to begin mail delivery");
        write_mailbox(&txn, character_id, &mails);
        txn.commit().expect("Failed to commit mail delivery");
    }
    delivered
}

// --- Systems ---

#[derive(SystemParam)]
struct MailDesk<'w, 's> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    gold: ResMut<'w, GoldUpdateQueue>,
    changed: ResMut<'w, ChangedMailboxes>,
    contents: Query<'w, 's, &'static mut MessageSender<MailboxContents>>,
    sent: Query<'w, 's, &'static mut MessageSender<MailSent>>,
    failures: Query<'w, 's, &'static mut MessageSender<MailFailed>>,
}

impl MailDesk<'_, '_> {
    fn stores(&self) -> Option<(&Database, &ItemTemplates)> {
        Some((&self.db.as_ref()?.db, self.templates.as_deref()?))
    }

    fn apply(&mut self, changes: MailChanges) {
        for (character_id, gold) in changes.gold {
            self.gold.record(character_id, gold);
        }
        self.changed.0.extend(changes.mailboxes);
    }

    fn fail(&mut self, connection: Entity, object: u64, error: MailError) {
        if let Ok(mut sender) = self.failures.get_mut(connection) {
            sender.send::<MailChannel>(MailFailed { object, error });
        }
    }

    fn send_contents(&mut self, connection: Entity, character_id: u64, object: Entity) {
        let Some((db, templates)) = self.stores() else {
            return;
        };
        let contents = mailbox_contents(db, templates, character_id, object.to_bits());
        if let Ok(mut sender) = self.contents.get_mut(connection) {
            sender.send::<MailChannel>(contents);
        }
    }
}

/// The mailbox whose frame is open on this connection.
fn open_mailbox(interaction: Option<&ActiveInteraction>) -> Option<Entity> {
    interaction
        .filter(|interaction| interaction.role == Some(NpcRole::Mailbox))
        .map(|interaction| interaction.npc)
}

type Requests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        Option<&'static ActiveInteraction>,
        &'static mut MessageReceiver<M>,
    ),
>;

/// Opening a mailbox sends the inbox (Retail `MAIL_SHOW` + `MAIL_INBOX_UPDATE`).
fn send_opened_mailboxes(
    opened: Query<(Entity, &InWorld, &ActiveInteraction), Changed<ActiveInteraction>>,
    mut desk: MailDesk,
) {
    for (connection, in_world, interaction) in &opened {
        if let Some(object) = open_mailbox(Some(interaction)) {
            desk.send_contents(connection, in_world.character_id, object);
        }
    }
}

fn process_send_requests(mut requests: Requests<SendMail>, mut desk: MailDesk) {
    for (connection, in_world, interaction, mut receiver) in &mut requests {
        let open = open_mailbox(interaction).map(Entity::to_bits);
        for request in receiver.receive() {
            let result = match desk.stores() {
                None => Err(MailError::Internal),
                Some(_) if open != Some(request.object) => Err(MailError::NotInteracting),
                Some((db, templates)) => {
                    send_mail(db, templates, in_world.character_id, &request, unix_now())
                }
            };
            match result {
                Ok(changes) => {
                    desk.apply(changes);
                    // TC a352b1fa MailHandler.cpp224: postage only, not attached money.
                    let postage = shared::mail::postage(request.attachments.len());
                    desk.gold.record_criteria(
                        in_world.character_id,
                        crate::achievement::CriteriaEvent::MoneySpentOnPostage(postage),
                    );
                    if let Ok(mut sender) = desk.sent.get_mut(connection) {
                        sender.send::<MailChannel>(MailSent {
                            object: request.object,
                        });
                    }
                }
                Err(error) => desk.fail(connection, request.object, error),
            }
        }
    }
}

fn process_mail_requests(mut requests: Requests<MailRequest>, mut desk: MailDesk) {
    for (connection, in_world, interaction, mut receiver) in &mut requests {
        let open = open_mailbox(interaction).map(Entity::to_bits);
        for request in receiver.receive() {
            let result = match desk.stores() {
                None => Err(MailError::Internal),
                Some(_) if open != Some(request.object) => Err(MailError::NotInteracting),
                Some((db, templates)) => act_on_mail(
                    db,
                    templates,
                    in_world.character_id,
                    request.mail_id,
                    request.action,
                    unix_now(),
                ),
            };
            match result {
                Ok(changes) => desk.apply(changes),
                Err(error) => desk.fail(connection, request.object, error),
            }
        }
    }
}

/// Expire old mail, then send `PendingMail` where the unread senders changed and
/// resend open mailboxes whose delivered mail changed.
fn tick_mail(
    mut commands: Commands,
    mut connections: Query<(Entity, &InWorld, Option<&mut SentPendingMail>)>,
    mut pending: Query<&mut MessageSender<PendingMail>>,
    mut desk: MailDesk,
) {
    let Some(db) = desk.db.as_ref().map(|db| &db.db) else {
        return;
    };
    let now = unix_now();
    let expired = expire_mail(db, now);
    let mut changed: Vec<u64> = expired;
    for (connection, in_world, sent) in &mut connections {
        let mails = load_mailbox(db, in_world.character_id);
        let senders = pending_senders(&mails, now);
        let delivered = inbox(&mails, now).len();
        let (last, last_delivered) = sent
            .as_ref()
            .map_or((None, 0), |sent| (sent.senders.as_ref(), sent.delivered));
        if last != Some(&senders) {
            if let Ok(mut sender) = pending.get_mut(connection) {
                sender.send::<MailChannel>(PendingMail {
                    senders: senders.clone(),
                });
            }
        }
        if sent.is_some() && last_delivered != delivered {
            changed.push(in_world.character_id);
        }
        let state = SentPendingMail {
            senders: Some(senders),
            delivered,
        };
        match sent {
            Some(mut sent) => *sent = state,
            None => {
                commands.entity(connection).insert(state);
            }
        }
    }
    desk.changed.0.extend(changed);
}

/// Resend the inbox to every open mailbox of a changed character.
fn send_changed_mailboxes(open: Query<(Entity, &InWorld, &ActiveInteraction)>, mut desk: MailDesk) {
    let changed = std::mem::take(&mut desk.changed.0);
    if changed.is_empty() {
        return;
    }
    for (connection, in_world, interaction) in &open {
        let Some(object) = open_mailbox(Some(interaction)) else {
            continue;
        };
        if changed.contains(&in_world.character_id) {
            desk.send_contents(connection, in_world.character_id, object);
        }
    }
}

#[cfg(test)]
#[path = "mail_tests.rs"]
mod tests;
