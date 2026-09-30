use super::*;
use shared::components::Gold;

#[derive(Resource, Default)]
struct MailState {
    inbox: Option<MailboxContents>,
    bags: MerchantState,
    expected: Option<ClaimProof>,
}

enum ClaimProof {
    Money {
        mail_id: u64,
        gold: u64,
    },
    Attachment {
        mail_id: u64,
        slot: u8,
        item_id: u32,
        count: u64,
    },
}

#[derive(SystemParam)]
struct MailIo<'w, 's> {
    use_object: Query<'w, 's, &'static mut MessageSender<UseGameObject>>,
    request: Query<'w, 's, &'static mut MessageSender<MailRequest>>,
    opened: Query<'w, 's, &'static mut MessageReceiver<InteractionOpened>>,
    failed: Query<'w, 's, &'static mut MessageReceiver<InteractionFailed>>,
    closed: Query<'w, 's, &'static mut MessageReceiver<InteractionClosed>>,
    inbox: Query<'w, 's, &'static mut MessageReceiver<MailboxContents>>,
    mail_failed: Query<'w, 's, &'static mut MessageReceiver<MailFailed>>,
    snapshot: Query<'w, 's, &'static mut MessageReceiver<InventorySnapshot>>,
    delta: Query<'w, 's, &'static mut MessageReceiver<InventoryDelta>>,
    managers: Query<'w, 's, &'static MessageManager, With<Connected>>,
    gold: Query<'w, 's, &'static Gold>,
    selected: Res<'w, SelectedCharacterId>,
}

impl MailIo<'_, '_> {
    fn own_gold(&self) -> Option<u64> {
        let remote = Entity::try_from_bits(self.selected.0?)?;
        self.managers.iter().find_map(|manager| {
            let local = manager.entity_mapper.remote_to_local.get(&remote)?;
            self.gold.get(*local).ok().map(|gold| gold.0)
        })
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<MailState>();
    app.add_systems(Update, mail_system);
}

fn mail_system(
    state: Res<State<CliState>>,
    action: Res<CliAction>,
    mut flow: ResMut<EconomyFlow>,
    mut mail: ResMut<MailState>,
    mut io: MailIo,
    mut exit: MessageWriter<AppExit>,
) {
    let CliCommand::Economy(action) = &action.command else {
        return;
    };
    if !action.operation.is_mail() || flow.phase == Phase::Done {
        return;
    }
    receive_mail_state(action.npc, &mut mail, &mut io);
    if *state.get() != CliState::InWorld {
        return;
    }
    if flow.phase == Phase::Idle {
        flow.phase = Phase::Interaction;
        flow.started = std::time::Instant::now();
        if let Err(error) =
            send::<_, InteractionChannel>(&mut io.use_object, UseGameObject { object: action.npc })
        {
            finish(&mut flow, &mut exit, Err(error));
            return;
        }
    }
    receive_mail_session(action.npc, &mut flow, &mut io, &mut exit);
    advance_mail(action, &mut flow, &mut mail, &mut io, &mut exit);
    if let Some(error) = flow.timeout(flow.started.elapsed()) {
        finish(&mut flow, &mut exit, Err(error));
    }
}

fn receive_mail_state(object: u64, mail: &mut MailState, io: &mut MailIo) {
    for mut receiver in &mut io.inbox {
        for contents in receiver.receive() {
            if contents.object == object {
                mail.inbox = Some(contents);
            }
        }
    }
    for mut receiver in &mut io.snapshot {
        for snapshot in receiver.receive() {
            mail.bags.snapshot(snapshot);
        }
    }
    for mut receiver in &mut io.delta {
        for delta in receiver.receive() {
            mail.bags.delta(delta);
        }
    }
}

fn receive_mail_session(
    object: u64,
    flow: &mut EconomyFlow,
    io: &mut MailIo,
    exit: &mut MessageWriter<AppExit>,
) {
    for mut receiver in &mut io.failed {
        for failure in receiver.receive() {
            if failure.npc == object {
                finish(flow, exit, Err(failure.error.message().into()));
            }
        }
    }
    for mut receiver in &mut io.closed {
        for closed in receiver.receive() {
            if closed.npc == object {
                finish(flow, exit, Err("Mailbox interaction closed".into()));
            }
        }
    }
    for mut receiver in &mut io.mail_failed {
        for failure in receiver.receive() {
            if failure.object == object {
                finish(flow, exit, Err(failure.error.message().into()));
            }
        }
    }
    receive_mail_role(object, flow, io, exit);
}

fn receive_mail_role(
    object: u64,
    flow: &mut EconomyFlow,
    io: &mut MailIo,
    exit: &mut MessageWriter<AppExit>,
) {
    for mut receiver in &mut io.opened {
        for opened in receiver.receive() {
            if opened.npc != object || flow.phase != Phase::Interaction {
                continue;
            }
            if opened.kind == InteractionKind::Role(NpcRole::Mailbox) {
                flow.phase = Phase::Request;
            } else {
                finish(
                    flow,
                    exit,
                    Err(format!("Expected Mailbox role, got {:?}", opened.kind)),
                );
            }
        }
    }
}

fn advance_mail(
    action: &EconomyAction,
    flow: &mut EconomyFlow,
    mail: &mut MailState,
    io: &mut MailIo,
    exit: &mut MessageWriter<AppExit>,
) {
    if flow.phase == Phase::Request {
        let Some(inbox) = &mail.inbox else {
            return;
        };
        if matches!(action.operation, EconomyOperation::MailList) {
            finish(flow, exit, Ok(format!("mailbox {inbox:#?}")));
            return;
        }
        if let Err(error) = send_mail_claim(action, flow, mail, io) {
            finish(flow, exit, Err(error));
        }
    }
    if flow.phase == Phase::Awaiting {
        if let Some(report) = confirm_claim(mail, io.own_gold()) {
            finish(flow, exit, Ok(report));
        }
    }
}

fn send_mail_claim(
    action: &EconomyAction,
    flow: &mut EconomyFlow,
    mail: &mut MailState,
    io: &mut MailIo,
) -> Result<(), String> {
    let Some((mail_id, claim, proof)) = prepare_claim(&action.operation, mail, io.own_gold())?
    else {
        return Ok(());
    };
    send::<_, MailChannel>(
        &mut io.request,
        MailRequest {
            object: action.npc,
            mail_id,
            action: claim,
        },
    )?;
    mail.expected = Some(proof);
    flow.phase = Phase::Awaiting;
    flow.started = std::time::Instant::now();
    Ok(())
}

fn prepare_claim(
    operation: &EconomyOperation,
    mail: &MailState,
    gold: Option<u64>,
) -> Result<Option<(u64, MailAction, ClaimProof)>, String> {
    let (mail_id, action) = match *operation {
        EconomyOperation::MailTakeMoney { mail_id } => (mail_id, MailAction::TakeMoney),
        EconomyOperation::MailTakeAttachment { mail_id, slot } => {
            (mail_id, MailAction::TakeAttachment { slot })
        }
        _ => return Ok(None),
    };
    let header = mail
        .inbox
        .as_ref()
        .and_then(|inbox| inbox.mails.iter().find(|header| header.mail_id == mail_id))
        .ok_or_else(|| format!("Mail {mail_id} is not in the delivered inbox"))?;
    let proof = match action {
        MailAction::TakeMoney => prepare_money_proof(header, gold)?,
        MailAction::TakeAttachment { slot } => prepare_attachment_proof(header, slot, &mail.bags)?,
        _ => unreachable!("only claim actions constructed above"),
    };
    Ok(proof.map(|proof| (mail_id, action, proof)))
}

fn prepare_money_proof(
    header: &MailHeader,
    gold: Option<u64>,
) -> Result<Option<ClaimProof>, String> {
    if header.money == 0 {
        return Err(format!("Mail {} has no money", header.mail_id));
    }
    let Some(gold) = gold else {
        return Ok(None);
    };
    let expected = gold
        .checked_add(header.money)
        .ok_or("Claim money overflow")?;
    Ok(Some(ClaimProof::Money {
        mail_id: header.mail_id,
        gold: expected,
    }))
}

fn prepare_attachment_proof(
    header: &MailHeader,
    slot: u8,
    bags: &MerchantState,
) -> Result<Option<ClaimProof>, String> {
    let attachment = header
        .attachments
        .iter()
        .find(|attachment| attachment.slot == slot)
        .ok_or_else(|| format!("Mail {} has no attachment slot {slot}", header.mail_id))?;
    if !bags.inventory_ready {
        return Ok(None);
    }
    let item_id = attachment.item.item_id;
    let count = bags.count(item_id) + u64::from(attachment.item.count);
    Ok(Some(ClaimProof::Attachment {
        mail_id: header.mail_id,
        slot,
        item_id,
        count,
    }))
}

fn confirm_claim(mail: &MailState, gold: Option<u64>) -> Option<String> {
    let inbox = mail.inbox.as_ref()?;
    match mail.expected.as_ref()? {
        ClaimProof::Money {
            mail_id,
            gold: expected,
        } => {
            let header = inbox
                .mails
                .iter()
                .find(|header| header.mail_id == *mail_id)?;
            if header.money != 0 || gold != Some(*expected) {
                return None;
            }
            Some(format!(
                "money claim confirmed by inbox and gold: mail_id={mail_id} gold={expected}"
            ))
        }
        ClaimProof::Attachment {
            mail_id,
            slot,
            item_id,
            count,
        } => {
            let header = inbox
                .mails
                .iter()
                .find(|header| header.mail_id == *mail_id)?;
            let still_attached = header
                .attachments
                .iter()
                .any(|attachment| attachment.slot == *slot);
            let bag_count = mail.bags.count(*item_id);
            if still_attached || bag_count < *count {
                return None;
            }
            Some(format!(
                "attachment claim confirmed by inbox and inventory: mail_id={mail_id} slot={slot} item_id={item_id} count={bag_count}"
            ))
        }
    }
}
