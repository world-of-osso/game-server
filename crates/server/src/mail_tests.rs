use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use shared::protocol::{
    MailAction, MailChannel, MailError, MailFailed, MailRequest, MailSent, MailboxContents,
    PendingMail, SendMail,
};

use super::*;
use crate::bank_test_harness::{Harness, LINEN, PEACEBLOOM, harness};
use crate::persistence::mail_store::{MailData, MailItemData, MailSender, load_mailbox};

const DAY: u64 = 86_400;

#[test]
fn toyfx66_katy_source_mailbox_npc_opens_mail_and_sends_without_gossip_substitution() {
    use shared::components::{Health, Npc, Position, UnitFactionTemplate};
    use shared::protocol::{
        InteractNpc, InteractionChannel, InteractionKind, InteractionOpened, NpcFlags, NpcRole,
    };
    let mut h = harness().unwrap();
    let recipient = h.create_alt("Katymailb");
    h.set_gold(100);
    let flags: u64 = h
        .conn
        .query_row(
            "SELECT npcflag FROM tdb_creature_template WHERE entry=132969",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(flags, NpcFlags::MAILBOX | NpcFlags::GOSSIP);
    let katy = h
        .app
        .world_mut()
        .spawn((
            Npc {
                template_id: 132969,
                name: "Katy Stampwhistle".into(),
            },
            NpcFlags(flags),
            Position {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitFactionTemplate(35),
            crate::maps::InMap::continent(0),
        ))
        .id();
    h.send::<_, InteractionChannel>(InteractNpc {
        npc: katy.to_bits(),
    });
    assert!(
        h.take::<InteractionOpened>()
            .iter()
            .any(|opened| opened.npc == katy.to_bits()
                && opened.kind == InteractionKind::Role(NpcRole::Mailbox)),
        "authentic mailbox flag opens mail, not an invented gossip option"
    );
    // As with the existing game-object fixture, contents follow the role by one frame.
    h.app.update();
    let contents = h.take::<MailboxContents>();
    assert!(
        contents
            .iter()
            .any(|mailbox| mailbox.object == katy.to_bits() && mailbox.mails.is_empty()),
        "contents={contents:?}, active={:?}",
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.connection)
    );
    send(&mut h, katy, letter("Katymailb"));
    assert!(failures(&mut h).is_empty());
    let mails = load_mailbox(h.db(), recipient);
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].subject, "Linen");
    assert_eq!(h.gold(), 70);
    h.move_player(30.0);
    send(&mut h, katy, letter("Katymailb"));
    assert!(failures(&mut h).contains(&MailError::NotInteracting));
    assert_eq!(load_mailbox(h.db(), recipient).len(), 1);
    assert_eq!(h.gold(), 70);
    h.move_player(0.0);
    h.send::<_, InteractionChannel>(InteractNpc {
        npc: katy.to_bits(),
    });
    assert!(
        h.take::<InteractionOpened>()
            .iter()
            .any(|opened| opened.kind == InteractionKind::Role(NpcRole::Mailbox))
    );
    h.app.world_mut().get_mut::<Health>(katy).unwrap().current = 0.0;
    for _ in 0..3 {
        h.app.update();
    }
    send(&mut h, katy, letter("Katymailb"));
    assert!(failures(&mut h).contains(&MailError::NotInteracting));
    assert_eq!(load_mailbox(h.db(), recipient).len(), 1);
    assert_eq!(h.gold(), 70);
}

#[test]
fn achievements_criteria_postage_and_most_money_follow_committed_mail_not_requests() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState};
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Posta").unwrap();
    let recipient = h.create_alt("Postb");
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerAchievementState::default());
    let progress = |h: &Harness, criterion: u32| {
        persistence::load_achievements(h.db(), h.character_id)
            .criteria_progress
            .get(&criterion)
            .copied()
    };
    let before = h.gold();
    let mailbox = h.open_mailbox();
    send(&mut h, mailbox, letter("Posta"));
    assert_eq!(failures(&mut h), vec![MailError::ToSelf]);
    assert_eq!(h.gold(), before);
    assert_eq!(progress(&h, 3358), None);
    assert_eq!(progress(&h, 4224), None);

    let linen = h.guid_of(LINEN);
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![linen],
            money: 5_000,
            ..letter("Postb")
        },
    );
    assert!(failures(&mut h).is_empty());
    assert_eq!(h.take::<MailSent>().len(), 1);
    assert_eq!(h.gold(), before - 5_030);
    assert_eq!(progress(&h, 3358), Some(30));
    assert_eq!(progress(&h, 4224), Some(before - 5_030));
    assert_eq!(load_mailbox(h.db(), recipient).len(), 1);

    send(&mut h, mailbox, letter("Postb"));
    assert!(failures(&mut h).is_empty());
    assert_eq!(h.gold(), before - 5_060);
    assert_eq!(progress(&h, 3358), Some(60));
    assert_eq!(progress(&h, 4224), Some(before - 5_030));
    for _ in 0..3 {
        h.app.update();
    }
    assert_eq!(progress(&h, 3358), Some(60));
    assert_eq!(progress(&h, 4224), Some(before - 5_030));
    assert!(
        !persistence::load_achievements(h.db(), recipient)
            .criteria_progress
            .contains_key(&3358)
    );
    let character = h.character_id;
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert_eq!(saved.criteria_progress.get(&3358), Some(&60));
    assert_eq!(saved.criteria_progress.get(&4224), Some(&(before - 5_030)));
}

fn send(h: &mut Harness, mailbox: Entity, mail: SendMail) {
    h.send::<_, MailChannel>(SendMail {
        object: mailbox.to_bits(),
        ..mail
    });
}

fn letter(recipient: &str) -> SendMail {
    SendMail {
        object: 0,
        recipient: recipient.into(),
        subject: "Linen".into(),
        body: "For your tailoring.".into(),
        attachments: Vec::new(),
        money: 0,
        cod: 0,
    }
}

fn act(h: &mut Harness, mailbox: Entity, mail_id: u64, action: MailAction) {
    h.send::<_, MailChannel>(MailRequest {
        object: mailbox.to_bits(),
        mail_id,
        action,
    });
}

fn failures(h: &mut Harness) -> Vec<MailError> {
    h.take::<MailFailed>()
        .into_iter()
        .map(|failed| failed.error)
        .collect()
}

fn templates(h: &Harness) -> &ItemTemplates {
    h.app.world().resource::<ItemTemplates>()
}

/// Advance the clock past one mail tick (virtual time steps at most 250 ms).
fn run_mail_tick(h: &mut Harness) {
    h.app
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            250,
        )));
    for _ in 0..6 {
        h.app.update();
    }
    h.app.insert_resource(TimeUpdateStrategy::Automatic);
}

#[test]
fn transmogacq_receiving_player_mail_keeps_boe_uncollected() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Maila").unwrap();
    let recipient = h.create_alt("Mailb");
    let mut inventory = persistence::load_inventory(h.db(), h.character_id);
    let guid = persistence::next_id(h.db(), "item_instance");
    inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id: 25,
        name: "Worn Shortsword".into(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(h.db(), h.character_id, &inventory);
    let mailbox = h.open_mailbox();
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![guid],
            ..letter("Mailb")
        },
    );
    assert!(failures(&mut h).is_empty());
    let mail_id = load_mailbox(h.db(), recipient)[0].mail_id;
    h.play_as(recipient);
    let mailbox = h.open_mailbox();
    act(
        &mut h,
        mailbox,
        mail_id,
        MailAction::TakeAttachment { slot: 0 },
    );
    assert!(failures(&mut h).is_empty());
    assert_eq!(h.count_of(25), 1);
    assert!(!crate::persistence::appearance_store::load_account_appearances(h.db(), 1).has(154));
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    persistence::create_tables(&reopened);
    assert!(!crate::persistence::appearance_store::load_account_appearances(&reopened, 1).has(154));
}

#[test]
fn item_and_money_to_another_account_arrive_after_an_hour_and_are_taken_at_the_mailbox() {
    let Some(mut h) = harness() else { return };
    let recipient = h.create_on_account(2, "Tradeb", 1);
    let linen = h.guid_of(LINEN);
    let gold_before = h.gold();
    let mailbox = h.open_mailbox();
    assert_eq!(h.take::<MailboxContents>().len(), 1);

    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![linen],
            money: 5_000,
            ..letter("tradeb")
        },
    );

    assert_eq!(h.take::<MailSent>().len(), 1, "{:?}", failures(&mut h));
    // Postage 30c for one item plus the 5,000c sent.
    assert_eq!(h.gold(), gold_before - 5_030);
    assert_eq!(h.count_of(LINEN), 0);
    let now = unix_now();
    let in_transit = load_mailbox(h.db(), recipient);
    assert_eq!(in_transit.len(), 1);
    assert!((now + 3_599..=now + 3_601).contains(&in_transit[0].delivers_at));
    assert_eq!(
        in_transit[0].expires_at,
        in_transit[0].delivers_at + 30 * DAY
    );
    assert!(inbox(&in_transit, now).is_empty());

    assert_eq!(deliver_pending_mail(h.db(), recipient, now), 1);
    h.play_as(recipient);
    run_mail_tick(&mut h);
    let pending = h.take::<PendingMail>();
    assert_eq!(pending.last().unwrap().senders, vec!["Banker".to_string()]);

    let mailbox = h.open_mailbox();
    let contents = h.take::<MailboxContents>().pop().unwrap();
    let mail = &contents.mails[0];
    assert_eq!(mail.sender, "Banker");
    assert_eq!(mail.money, 5_000);
    assert_eq!(mail.attachments[0].item.item_id, LINEN);
    assert_eq!(mail.attachments[0].item.count, 20);
    assert!(mail.returnable && !mail.can_delete());

    let recipient_gold = h.gold();
    let recipient_linen = h.count_of(LINEN);
    act(
        &mut h,
        mailbox,
        mail.mail_id,
        MailAction::TakeAttachment { slot: 0 },
    );
    act(&mut h, mailbox, mail.mail_id, MailAction::TakeMoney);

    assert!(failures(&mut h).is_empty());
    assert_eq!(h.count_of(LINEN), recipient_linen + 20);
    assert_eq!(h.gold(), recipient_gold + 5_000);
    let emptied = h.take::<MailboxContents>().pop().unwrap();
    assert!(emptied.mails[0].attachments.is_empty());
    assert_eq!(emptied.mails[0].money, 0);
    assert!(emptied.mails[0].read && emptied.mails[0].can_delete());
    run_mail_tick(&mut h);
    assert_eq!(
        h.take::<PendingMail>().pop().unwrap().senders,
        Vec::<String>::new()
    );
}

#[test]
fn cod_is_paid_with_the_first_item_and_mailed_to_the_sender() {
    let Some(mut h) = harness() else { return };
    let seller = h.character_id;
    let buyer = h.create_alt("Buyer");
    let peacebloom = h.guid_of(PEACEBLOOM);
    let mailbox = h.open_mailbox();
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![peacebloom],
            cod: 2_000,
            ..letter("Buyer")
        },
    );
    assert_eq!(h.take::<MailSent>().len(), 1);
    let mail = &load_mailbox(h.db(), buyer)[0];
    // Same account: no delivery delay; C.O.D. mail lasts 3 days.
    assert_eq!(mail.expires_at, mail.delivers_at + 3 * DAY);
    let mail_id = mail.mail_id;

    h.play_as(buyer);
    let mailbox = h.open_mailbox();
    h.set_gold(1_999);
    act(
        &mut h,
        mailbox,
        mail_id,
        MailAction::TakeAttachment { slot: 0 },
    );
    assert_eq!(failures(&mut h), vec![MailError::CodInsufficientMoney]);
    let peacebloom_before = h.count_of(PEACEBLOOM);
    h.set_gold(5_000);
    act(
        &mut h,
        mailbox,
        mail_id,
        MailAction::TakeAttachment { slot: 0 },
    );

    assert!(failures(&mut h).is_empty());
    assert_eq!(h.gold(), 3_000);
    assert_eq!(h.count_of(PEACEBLOOM), peacebloom_before + 10);
    let payments = load_mailbox(h.db(), seller);
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].money, 2_000);
    assert!(payments[0].cod_payment && !payments[0].returnable());
    assert_eq!(
        payments[0].sender,
        MailSender::Player {
            character_id: buyer,
            name: "Buyer".into()
        }
    );
}

#[test]
fn refused_sends_change_nothing() {
    let Some(mut h) = harness() else { return };
    h.create_alt("Buyer");
    h.create_on_account(2, "Grunt", 2);
    let linen = h.guid_of(LINEN);
    let gold = h.gold();

    // No open mailbox.
    send(&mut h, Entity::from_bits(1), letter("Buyer"));
    let mailbox = h.open_mailbox();
    send(&mut h, mailbox, letter("Banker"));
    send(&mut h, mailbox, letter("Nobody"));
    send(&mut h, mailbox, letter("Grunt"));
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![linen],
            money: 10,
            cod: 10,
            ..letter("Buyer")
        },
    );
    h.make_soulbound(linen);
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![linen],
            ..letter("Buyer")
        },
    );
    h.set_gold(29);
    send(&mut h, mailbox, letter("Buyer"));

    assert_eq!(
        failures(&mut h),
        vec![
            MailError::NotInteracting,
            MailError::ToSelf,
            MailError::RecipientNotFound,
            MailError::WrongFaction,
            MailError::Internal,
            MailError::BoundItem,
            MailError::NotEnoughMoney,
        ]
    );
    assert!(h.take::<MailSent>().is_empty());
    assert_eq!(h.count_of(LINEN), 20);
    h.set_gold(gold);
}

#[test]
fn returned_mail_goes_back_once_and_letters_delete() {
    let Some(mut h) = harness() else { return };
    let sender = h.character_id;
    let recipient = h.create_alt("Buyer");
    let peacebloom = h.guid_of(PEACEBLOOM);
    let mailbox = h.open_mailbox();
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![peacebloom],
            ..letter("Buyer")
        },
    );
    send(&mut h, mailbox, letter("Buyer"));
    let mails = load_mailbox(h.db(), recipient);
    let (parcel, text) = (mails[0].mail_id, mails[1].mail_id);

    h.play_as(recipient);
    let mailbox = h.open_mailbox();
    act(&mut h, mailbox, parcel, MailAction::Delete);
    assert_eq!(failures(&mut h), vec![MailError::CannotDelete]);
    act(&mut h, mailbox, parcel, MailAction::Return);
    act(&mut h, mailbox, text, MailAction::Delete);
    assert!(failures(&mut h).is_empty());
    assert!(load_mailbox(h.db(), recipient).is_empty());

    let back = load_mailbox(h.db(), sender);
    assert_eq!(back.len(), 1);
    assert!(back[0].returned && !back[0].returnable());
    assert_eq!(back[0].items[0].item.item_guid, peacebloom);
    // Same account: no delivery delay for the returned items.
    assert!(back[0].is_delivered(unix_now()));
    h.play_as(sender);
    let mailbox = h.open_mailbox();
    act(&mut h, mailbox, back[0].mail_id, MailAction::Return);
    assert_eq!(failures(&mut h), vec![MailError::Internal]);
}

fn parcel(mail_id: u64, sender: MailSender, delivered: u64) -> MailData {
    MailData {
        mail_id,
        sender,
        subject: "Parcel".into(),
        body: String::new(),
        items: vec![MailItemData {
            slot: 0,
            item: persistence::InventoryItemData {
                item_guid: 9_000 + mail_id,
                item_id: LINEN,
                name: "Linen Cloth".into(),
                quality: 1,
                required_level: 0,
                stack_count: 5,
                vendor_sell_price: 13,
                unique_auction: false,
            },
        }],
        money: 0,
        cod: 0,
        delivers_at: delivered,
        expires_at: delivered + 30 * DAY,
        read: false,
        returned: false,
        cod_payment: false,
    }
}

#[test]
fn expired_player_mail_is_returned_and_auction_mail_deleted() {
    let Some(h) = harness() else { return };
    let sender = h.character_id;
    let db = h.db();
    let (recipient, _) =
        persistence::create_character_with_test_bags(db, 2, "Tradeb", 1, 1).unwrap();
    let from = MailSender::Player {
        character_id: sender,
        name: "Banker".into(),
    };
    mail_store::deliver_mail(db, recipient, parcel(1, from, 1_000));
    mail_store::deliver_mail(db, recipient, parcel(2, MailSender::AuctionHouse, 1_000));
    mail_store::deliver_mail(db, recipient, parcel(3, MailSender::AuctionHouse, 5_000));

    let changed = expire_mail(db, 1_000 + 30 * DAY);

    assert!(changed.contains(&sender) && changed.contains(&recipient));
    let kept: Vec<u64> = load_mailbox(db, recipient)
        .iter()
        .map(|m| m.mail_id)
        .collect();
    assert_eq!(kept, vec![3]);
    let back = load_mailbox(db, sender);
    assert_eq!(back.len(), 1);
    assert!(back[0].returned);
    assert_eq!(back[0].items[0].item.item_guid, 9_001);
    // TrinityCore `MailDraft::SendReturnToSender`: items going back to another
    // account's character wait the delivery delay.
    let expired_at = 1_000 + 30 * DAY;
    assert_eq!(back[0].delivers_at, expired_at + 3_600);
    assert_eq!(back[0].expires_at, back[0].delivers_at + 30 * DAY);
    assert_eq!(
        back[0].sender,
        MailSender::Player {
            character_id: recipient,
            name: "Tradeb".into()
        }
    );
}

#[test]
fn taking_money_past_the_gold_cap_is_refused() {
    let Some(mut h) = harness() else { return };
    let mut mail = parcel(1, MailSender::AuctionHouse, 0);
    mail.items.clear();
    mail.money = 10;
    mail_store::deliver_mail(h.db(), h.character_id, mail);
    h.set_gold(MAX_MONEY - 5);

    let result = act_on_mail(
        h.db(),
        templates(&h),
        h.character_id,
        1,
        MailAction::TakeMoney,
        1,
    );

    assert_eq!(result, Err(MailError::TooMuchGold));
    assert_eq!(h.gold(), MAX_MONEY - 5);
    assert_eq!(load_mailbox(h.db(), h.character_id)[0].money, 10);
}

#[test]
fn pending_senders_are_the_newest_three_unread_delivered() {
    let player = |id, name: &str| MailSender::Player {
        character_id: id,
        name: name.into(),
    };
    let mut read = parcel(5, player(5, "Reader"), 500);
    read.read = true;
    let mails = vec![
        parcel(1, player(1, "Old"), 100),
        parcel(2, MailSender::AuctionHouse, 200),
        parcel(3, player(3, "Tradea"), 300),
        parcel(4, player(3, "Tradea"), 400),
        read,
        parcel(6, player(6, "Later"), 9_999),
    ];

    assert_eq!(
        pending_senders(&mails, 1_000),
        vec!["Tradea".to_string(), "Auction House".into(), "Old".into()]
    );
}

#[test]
fn a_mailbox_takes_mail_until_it_holds_more_than_a_hundred() {
    // TrinityCore `HandleSendMail`: `mailsCount > 100` is MAIL_ERR_RECIPIENT_CAP_REACHED.
    let Some(mut h) = harness() else { return };
    let recipient = h.create_alt("Buyer");
    let from = MailSender::Player {
        character_id: h.character_id,
        name: "Banker".into(),
    };
    for mail_id in 1..=100 {
        mail_store::deliver_mail(h.db(), recipient, parcel(mail_id, from.clone(), 1_000));
    }
    let mailbox = h.open_mailbox();
    send(&mut h, mailbox, letter("Buyer"));
    assert_eq!(h.take::<MailSent>().len(), 1, "{:?}", failures(&mut h));
    send(&mut h, mailbox, letter("Buyer"));
    assert_eq!(failures(&mut h), vec![MailError::RecipientMailboxFull]);
    assert_eq!(load_mailbox(h.db(), recipient).len(), 101);
}

#[test]
fn item_mail_between_guild_members_arrives_at_once() {
    // TrinityCore `HandleSendMail`: "Mail sent between guild members arrives instantly".
    let mut h = harness().expect("Actual same-guild mail fixture assets are required");
    let recipient = h.create_on_account(2, "Tradeb", 1);
    for member in [h.character_id, recipient] {
        persistence::set_character_guild_by_id(h.db(), member, 7, "Postmasters").unwrap();
    }
    let linen = h.guid_of(LINEN);
    let mailbox = h.open_mailbox();
    send(
        &mut h,
        mailbox,
        SendMail {
            attachments: vec![linen],
            ..letter("Tradeb")
        },
    );
    assert_eq!(h.take::<MailSent>().len(), 1, "{:?}", failures(&mut h));
    assert!(load_mailbox(h.db(), recipient)[0].is_delivered(unix_now()));
}
