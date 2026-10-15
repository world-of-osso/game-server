//! Targeted source Say is a nearby broadcast, not a whisper to the attacker.
use super::*;
use shared::components::Npc;

#[test]
fn encounter_coverage_wailing_aggro_speech_names_target_and_keeps_say_audience() {
    let (mut h, enemy, map) = bound_boss_at_difficulty(3669, 43, 1, 1, None);
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (disciple, _) = spawn_authored_actor(&mut h.app, &conn, 3678, map, 1);
    let origin = *h.app.world().get::<Position>(disciple).unwrap();
    let enemy_name = h.app.world().get::<Npc>(enemy).unwrap().name.clone();
    let near = h.player();
    let other = h.players[1].0;
    for listener in [near, other] {
        h.app.world_mut().entity_mut(listener).insert(origin);
    }
    // Do not run combat or fabricate an escort activation. The source hook is
    // dispatched with its actual NPC attacker, which is not itself a listener.
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(ReactState::Passive);
    h.advance(0.05);
    let wow = [origin.x, -origin.z, origin.y, 0.0];
    for (id, offset) in [(3, 25.0), (4, 25.01)] {
        let (player, connection) = spawn_player(&mut h.app, map, wow, id);
        h.app.world_mut().entity_mut(player).insert(Position {
            x: origin.x + offset,
            ..origin
        });
        h.players.push((player, connection, Inbox::default()));
    }
    let source = h.app.world().resource::<CreatureTexts>().groups[&3678][&10].clone();
    assert_eq!(source.len(), 5, "all five authored aggro alternatives");
    assert!(source.iter().any(|text| text.male_text.contains("$n")));
    let expected: Vec<_> = source
        .iter()
        .map(|text| text.male_text.replace("$n", &enemy_name))
        .collect();
    for (_, _, inbox) in &mut h.players {
        inbox.chat.clear();
    }
    let mut spoken = 0;
    let mut quiet = 0;
    let mut named = false;
    // Exercise the actual random production branch. Missing either outcome in
    // 128 independent 30% rolls has probability below 1.5e-20. This is not an
    // exact probability test; source review owns the literal 30% threshold.
    for _ in 0..128 {
        dispatch(h.app.world_mut(), disciple, Hook::JustEngagedWith(enemy));
        h.advance(0.1); // Host transport transfers MessageSender into MessageReceiver.
        let messages = std::mem::take(&mut h.players[0].2.chat);
        let edge = std::mem::take(&mut h.players[2].2.chat);
        let contents = |chat: &[ChatMessage]| {
            chat.iter()
                .map(|message| {
                    (
                        message.channel.clone(),
                        message.sender.clone(),
                        message.content.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            contents(&edge),
            contents(&messages),
            "25yd same-copy listener hears the Say"
        );
        assert!(h.inbox(1).chat.is_empty(), "other copy cannot hear");
        assert!(h.inbox(3).chat.is_empty(), "25.01yd cannot hear");
        assert!(messages.len() <= 1, "at most one Say per engagement");
        if messages.is_empty() {
            quiet += 1;
        }
        for message in messages {
            assert_eq!(message.channel, ChatType::MonsterSay(disciple.to_bits()));
            assert_eq!(message.sender, "Disciple of Naralex");
            assert!(expected.contains(&message.content), "{}", message.content);
            assert!(!message.content.contains("$n"));
            named |= message.content.contains(&enemy_name);
            spoken += 1;
        }
    }
    assert!(spoken > 0, "source aggro hook must sometimes speak");
    assert!(quiet > 0, "source aggro hook must sometimes stay silent");
    assert!(named, "NPC attacker name replaces $n for nearby hearers");
}
