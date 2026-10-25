//! Katy156833: authentic Assistant35 summon and live mailbox requests, not scenery-only.
use super::*;
use shared::protocol::{
    InteractNpc, InteractionChannel, InteractionKind, InteractionOpened, MailAction, MailChannel,
    MailError, MailFailed, MailRequest, MailboxContents, NpcRole,
};

fn install_katy_mailbox_service(h: &mut Harness, conn: &rusqlite::Connection) {
    h.app
        .insert_resource(crate::npc_interaction::load_gossip_catalog(conn).unwrap());
    bevy::app::Plugin::build(&crate::npc_interaction::NpcInteractionPlugin, &mut h.app);
    bevy::app::Plugin::build(&crate::mail::MailPlugin, &mut h.app);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<InteractNpc>::default(),
        MessageReceiver::<InteractNpc>::default(),
        MessageSender::<InteractionOpened>::default(),
        MessageReceiver::<InteractionOpened>::default(),
        MessageSender::<MailboxContents>::default(),
        MessageReceiver::<MailboxContents>::default(),
        MessageSender::<MailRequest>::default(),
        MessageReceiver::<MailRequest>::default(),
        MessageSender::<MailFailed>::default(),
        MessageReceiver::<MailFailed>::default(),
    ));
}

fn open_katy_mailbox(
    h: &mut Harness,
    actor: Entity,
) -> (Vec<InteractionOpened>, Vec<MailboxContents>) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<InteractNpc>>(h.connection)
        .unwrap()
        .send::<InteractionChannel>(InteractNpc {
            npc: actor.to_bits(),
        });
    let mut opened = Vec::new();
    let mut contents = Vec::new();
    for _ in 0..7 {
        h.advance(1);
        opened.extend(drain::<InteractionOpened>(&mut h.app, h.connection));
        contents.extend(drain::<MailboxContents>(&mut h.app, h.connection));
    }
    (opened, contents)
}

#[test]
fn toyfx66_katy_real_learning_summon_mailbox_service_and_source_expiry() {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Health, ModelDisplay, Npc, UnitIsPet};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let (mut h, character) = setup(&directory.path().join("katy.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    install_katy_mailbox_service(&mut h, &conn);
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, 156833);
    assert_eq!(
        activate(&mut h, 156833)[0].error,
        None,
        "actual source Assistant35 use"
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cooldown| cooldown.spell_id == 261602
                && !cooldown.is_gcd
                && cooldown.duration_ms == 10_800_000
                && cooldown.category == 2066)
    );
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 132969 && summon.spell_id == 261602)
        .map(|(entity, _, summon)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(
        actors.len(),
        1,
        "one authentic Katy, not a companion replacement"
    );
    let (actor, expires_at) = actors[0];
    assert_eq!(world.get::<ModelDisplay>(actor).unwrap().display_id, 68512);
    assert_eq!(
        world.get::<shared::protocol::NpcFlags>(actor),
        Some(&shared::protocol::NpcFlags(
            shared::protocol::NpcFlags::GOSSIP | shared::protocol::NpcFlags::MAILBOX
        ))
    );
    assert_eq!(
        world.get::<shared::components::UnitFactionTemplate>(actor),
        Some(&shared::components::UnitFactionTemplate(35))
    );
    assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(actor)
            .is_none()
    );
    let actor_position = *world.get::<Position>(actor).unwrap();
    let player_position = *world.get::<Position>(caster).unwrap();
    assert!(
        (actor_position.x - player_position.x - 3.0).abs() < 0.01,
        "source47 radius3 front placement"
    );
    assert!((actor_position.z - player_position.z).abs() < 0.01);
    let remaining = expires_at - crate::auras::aura_now(world);
    assert!(
        (599.0..=600.0).contains(&remaining),
        "source600s summon: {remaining}"
    );
    let (opened, contents) = open_katy_mailbox(&mut h, actor);
    assert_eq!(
        opened.last().map(|event| &event.kind),
        Some(&InteractionKind::Role(NpcRole::Mailbox)),
        "live source actor must open mailbox: actor={:?}, player={:?}, actor_map={:?}, player_map={:?}, combat={:?}, health={:?}, active={:?}",
        h.app.world().get::<Position>(actor),
        h.app.world().get::<Position>(caster),
        h.app.world().get::<crate::maps::InMap>(actor),
        h.app.world().get::<crate::maps::InMap>(caster),
        h.app.world().get::<CombatStatus>(actor),
        h.app.world().get::<Health>(actor),
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.connection)
    );
    assert!(
        contents
            .iter()
            .any(|mailbox| mailbox.object == actor.to_bits() && mailbox.mails.is_empty()),
        "live empty persisted mailbox must be delivered"
    );
    h.app.world_mut().get_mut::<Position>(caster).unwrap().x += 30.0;
    h.advance(7);
    assert!(
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.connection)
            .is_none()
    );
    assert!(
        open_katy_mailbox(&mut h, actor).0.is_empty(),
        "distance guard remains active"
    );
    assert_eq!(h.app.world().get::<Position>(actor), Some(&actor_position));
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = player_position;
    assert_eq!(
        open_katy_mailbox(&mut h, actor).0.last().unwrap().kind,
        InteractionKind::Role(NpcRole::Mailbox)
    );
    let remaining = expires_at - crate::auras::aura_now(h.app.world());
    h.advance((remaining / 0.05).ceil() as usize + 2);
    assert!(h.app.world().get_entity(actor).is_err());
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::npc_interaction::ActiveInteraction>(h.connection)
            .is_none()
    );
    assert!(
        open_katy_mailbox(&mut h, actor).0.is_empty(),
        "expired source actor cannot reopen mail"
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<MailRequest>>(h.connection)
        .unwrap()
        .send::<MailChannel>(MailRequest {
            object: actor.to_bits(),
            mail_id: 1,
            action: MailAction::TakeMoney,
        });
    let mut failures = Vec::new();
    for _ in 0..7 {
        h.advance(1);
        failures.extend(drain::<MailFailed>(&mut h.app, h.connection));
    }
    assert!(failures.iter().any(
        |failed| failed.object == actor.to_bits() && failed.error == MailError::NotInteracting
    ));
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
    assert_eq!(
        activate(&mut h, 156833)[0].error.as_deref(),
        Some("OnCooldown")
    );
}
