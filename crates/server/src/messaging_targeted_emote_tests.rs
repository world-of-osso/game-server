use super::*;

fn send_emote(app: &mut App, connection: Entity, emote: EmoteKind, target: Option<Entity>) {
    app.world_mut()
        .get_mut::<MessageSender<EmoteIntent>>(connection)
        .unwrap()
        .send::<ChatChannel>(EmoteIntent {
            emote,
            target_entity: target.map(Entity::to_bits),
        });
}

fn advance(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn love_counts(app: &App, player: Entity) -> [Option<u64>; 2] {
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    [3615, 3739].map(|criterion| state.criteria_progress.get(&criterion).copied())
}

#[test]
fn achievements_criteria_native_single_some_target_keeps_wave_and_love_qualified() {
    let (file, mut app, player, connection, character) = emote_app();
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    let spawn = |app: &mut App, template_id, map| {
        app.world_mut()
            .spawn((
                Npc {
                    template_id,
                    name: "Controlled target unit".into(),
                },
                map,
            ))
            .id()
    };
    // Authenticated69933 target IDs; this fixture proves selector semantics, not NPC asset closure.
    let slime = spawn(&mut app, 216058, crate::maps::InMap::continent(0));
    let wave_target = spawn(&mut app, 224734, crate::maps::InMap::continent(0));
    let foreign_slime = spawn(&mut app, 216058, crate::maps::InMap::continent(1));
    for target in [None, Some(player), Some(wave_target), Some(foreign_slime)] {
        send_emote(&mut app, connection, EmoteKind::Love, target);
    }
    send_emote(&mut app, connection, EmoteKind::Wave, Some(slime));
    advance(&mut app);
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert!(!state.criteria_progress.contains_key(&68751));
    assert!(!state.criteria_progress.contains_key(&3236));
    send_emote(&mut app, connection, EmoteKind::Love, Some(slime));
    send_emote(&mut app, connection, EmoteKind::Wave, Some(wave_target));
    advance(&mut app);
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&68751), Some(&1));
    assert_eq!(state.criteria_progress.get(&3236), Some(&1));
    for entry in [
        217559, 220413, 219842, 220168, 219366, 217316, 223663, 220370, 214726, 220177, 217461,
        220369, 221146, 220173, 217756, 219585, 226626,
    ] {
        let target = spawn(&mut app, entry, crate::maps::InMap::continent(0));
        send_emote(&mut app, connection, EmoteKind::Love, Some(target));
    }
    advance(&mut app);
    assert!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&40475)
    );
    advance(&mut app);
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let stored = persistence::load_achievements(&reopened, character);
    assert_eq!(stored.criteria_progress.get(&68751), Some(&1));
    assert_eq!(stored.criteria_progress.get(&3236), Some(&1));
}

#[test]
fn achievements_criteria_native_targeted_emote_uses_request_entity_and_persists() {
    let (file, mut app, player, connection, character) = emote_app();
    let spawn_npc = |app: &mut App, template_id, map| {
        app.world_mut()
            .spawn((
                Npc {
                    template_id,
                    name: "Critter".into(),
                },
                map,
            ))
            .id()
    };
    let squirrel = spawn_npc(&mut app, 1412, crate::maps::InMap::continent(0));
    let mammoth = spawn_npc(&mut app, 25679, crate::maps::InMap::continent(0));
    let foreign_squirrel = spawn_npc(&mut app, 1412, crate::maps::InMap::continent(1));
    let stale = spawn_npc(&mut app, 1412, crate::maps::InMap::continent(0));
    app.world_mut().despawn(stale);
    // A template ID is never accepted in place of a server entity ID.
    let forged_template = Entity::from_bits(1412);
    send_emote(&mut app, connection, EmoteKind::Love, Some(squirrel));
    advance(&mut app);
    assert_eq!(love_counts(&app, player), [None, None], "not in world");
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    // Current CombatTarget cannot substitute for any individual request's target.
    app.world_mut()
        .entity_mut(player)
        .insert(CombatTarget(Some(mammoth)));
    for target in [
        None,
        Some(player),
        Some(foreign_squirrel),
        Some(stale),
        Some(forged_template),
    ] {
        send_emote(&mut app, connection, EmoteKind::Love, target);
    }
    send_emote(&mut app, connection, EmoteKind::Wave, Some(squirrel));
    advance(&mut app);
    assert_eq!(
        love_counts(&app, player),
        [None, None],
        "unqualified targets/emote"
    );
    // Both messages travel through the real channel in the same tick.
    send_emote(&mut app, connection, EmoteKind::Love, Some(squirrel));
    send_emote(&mut app, connection, EmoteKind::Love, Some(mammoth));
    advance(&mut app);
    assert_eq!(love_counts(&app, player), [Some(1), Some(1)]);
    advance(&mut app);
    assert_eq!(
        love_counts(&app, player),
        [Some(1), Some(1)],
        "no idle replay"
    );
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let stored = persistence::load_achievements(&reopened, character);
    assert_eq!(stored.criteria_progress.get(&3615), Some(&1));
    assert_eq!(stored.criteria_progress.get(&3739), Some(&1));
}
