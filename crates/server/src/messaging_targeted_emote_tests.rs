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
fn achievements_criteria_native_some_target_accepts_either_request_unit_and_persists() {
    // Pinned69933 criterion20817 accepts63288 OR64798, never both on one unit.
    for entry in [63288, 64798] {
        let (file, mut app, player, connection, character) = emote_app();
        app.world_mut().entity_mut(connection).insert(InWorld {
            character_id: character,
        });
        let spawn = |app: &mut App, template_id, map| {
            app.world_mut()
                .spawn((
                    Npc {
                        template_id,
                        name: "Controlled sourced selector target".into(),
                    },
                    map,
                ))
                .id()
        };
        let accepted = spawn(&mut app, entry, crate::maps::InMap::continent(0));
        let wrong = spawn(&mut app, 1412, crate::maps::InMap::continent(0));
        let foreign = spawn(&mut app, entry, crate::maps::InMap::continent(1));
        for target in [None, Some(player), Some(wrong), Some(foreign)] {
            send_emote(&mut app, connection, EmoteKind::Love, target);
        }
        send_emote(&mut app, connection, EmoteKind::Wave, Some(accepted));
        advance(&mut app);
        assert!(
            !app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .contains_key(&20817)
        );
        // Final selection must not replace the target carried by this request.
        app.world_mut()
            .entity_mut(player)
            .insert(CombatTarget(Some(wrong)));
        send_emote(&mut app, connection, EmoteKind::Love, Some(accepted));
        advance(&mut app);
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&20817),
            Some(&1),
            "source alternative {entry}"
        );
        // All six authored Wave selectors for account40624; OR groups include
        // multiple alternatives and one duplicate entry, not fabricated credit.
        for spider in [224734, 227217, 216213, 224061, 220568, 217468] {
            let target = spawn(&mut app, spider, crate::maps::InMap::continent(0));
            send_emote(&mut app, connection, EmoteKind::Wave, Some(target));
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
                .contains(&40624)
        );
        advance(&mut app);
        let closed = app.world_mut().remove_resource::<GameDatabase>().unwrap();
        drop(closed);
        let reopened = redb::Database::open(file.path()).unwrap();
        // Enter-world loads character contributions before the account aggregate.
        let restored = persistence::load_achievements(&reopened, character);
        assert_eq!(restored.criteria_progress.get(&20817), Some(&1));
        app.insert_resource(GameDatabase::new(reopened));
        app.world_mut().entity_mut(player).insert(restored);
        app.world_mut().entity_mut(connection).remove::<InWorld>();
        app.world_mut().entity_mut(connection).insert(InWorld {
            character_id: character,
        });
        advance(&mut app);
        assert!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .account
                .as_deref()
                .unwrap()
                .earned_ids
                .contains(&40624)
        );
        drop(app);
        let reopened = redb::Database::open(file.path()).unwrap();
        let stored = persistence::load_achievements(&reopened, character);
        assert_eq!(stored.criteria_progress.get(&20817), Some(&1));
        assert_eq!(stored.criteria_progress.get(&68973), Some(&1));
        assert_eq!(stored.criteria_progress.get(&68976), Some(&1));
        assert_eq!(stored.criteria_progress.get(&68977), Some(&1));
    }
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

#[derive(Resource, Default)]
struct HugReceipts(Vec<EmoteEvent>);

fn collect_hug_receipts(
    mut receivers: Query<&mut MessageReceiver<EmoteEvent>>,
    mut receipts: ResMut<HugReceipts>,
) {
    for mut receiver in &mut receivers {
        receipts.0.extend(receiver.receive());
    }
}

fn spawn_hug_player(app: &mut App, race: u8, character: u64) -> Entity {
    use shared::components::{Health, Position, UnitFactionTemplate};
    app.world_mut()
        .spawn((
            Player {
                name: format!("Hug target {character}"),
                race,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
            ClientId(character),
            crate::maps::InMap::continent(0),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            UnitFactionTemplate(crate::faction::faction_template_for_race(race)),
            Health {
                current: 100.0,
                max: 100.0,
            },
            PlayerAchievementState::default(),
        ))
        .id()
}

fn kill_hug_players(app: &mut App, targets: [Entity; 2]) {
    for target in targets {
        assert!(crate::death::apply_health_damage(
            &mut app
                .world_mut()
                .get_mut::<shared::components::Health>(target)
                .unwrap(),
            100.0,
        ));
        app.world_mut()
            .write_message(crate::death::PlayerDeathTransition { player: target });
    }
    advance(app);
    for target in targets {
        let death = app
            .world()
            .get::<crate::death::PlayerDeathState>(target)
            .unwrap();
        assert_eq!(death.state, shared::death::DeathState::Dead);
        assert!(death.corpse.is_some());
    }
}

#[test]
fn achievements_criteria_hug247_native_dead_hostile_player_and_persistence() {
    use shared::components::{Health, Position, UnitFactionTemplate};
    let (file, mut app, player, connection, character) = emote_app();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    app.insert_resource(crate::faction::load_faction_templates(&conn).unwrap());
    // Lightyear clears receivers in Last; capture actual HostClient delivery in First.
    app.init_resource::<HugReceipts>();
    app.add_systems(First, collect_hug_receipts);
    app.init_resource::<crate::death::DeathServerState>();
    app.init_resource::<crate::death::DeathTransitionCounters>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.add_systems(
        Update,
        crate::death::process_player_death_transitions.before(process_chat_messages),
    );
    let map = crate::maps::InMap::continent(0);
    app.world_mut().entity_mut(player).insert((
        UnitFactionTemplate(1),
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    let enemy = spawn_hug_player(&mut app, 2, 9001);
    let friendly = spawn_hug_player(&mut app, 1, 9002);
    // Native damage and death transition create corpses, not hand-written predicate facts.
    kill_hug_players(&mut app, [enemy, friendly]);
    assert!(crate::spell_cast::targets::is_hostile(
        app.world(),
        player,
        enemy
    ));
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "not in world"
    );
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    let npc = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 1412,
                name: "Dead NPC".into(),
            },
            map,
            UnitFactionTemplate(2),
            Health {
                current: 0.0,
                max: 100.0,
            },
        ))
        .id();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    for target in [None, Some(friendly), Some(npc), Some(stale), Some(player)] {
        send_emote(&mut app, connection, EmoteKind::Hug, target);
    }
    send_emote(&mut app, connection, EmoteKind::Love, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379)
    );
    // Each current authority must hold; restored source values precede the valid receipt.
    app.world_mut()
        .entity_mut(enemy)
        .insert(crate::maps::InMap::continent(1));
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "foreign map"
    );
    app.world_mut()
        .entity_mut(enemy)
        .insert(crate::maps::InMap {
            map_id: 0,
            instance_id: 100,
        });
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "foreign copy of the same map"
    );
    app.world_mut().entity_mut(enemy).insert(map);
    app.world_mut().get_mut::<Health>(enemy).unwrap().current = 100.0;
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "living enemy"
    );
    app.world_mut().get_mut::<Health>(enemy).unwrap().current = 0.0;
    app.world_mut()
        .entity_mut(enemy)
        .remove::<UnitFactionTemplate>();
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "unknown hostility"
    );
    app.world_mut()
        .entity_mut(enemy)
        .insert(UnitFactionTemplate(2));
    app.world_mut().entity_mut(enemy).remove::<Health>();
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&2379),
        "unknown death authority"
    );
    app.world_mut().entity_mut(enemy).insert(Health {
        current: 0.0,
        max: 100.0,
    });
    app.world_mut().resource_mut::<HugReceipts>().0.clear();
    // Selection is deliberately wrong; only the message's target qualifies.
    app.world_mut()
        .entity_mut(player)
        .insert(CombatTarget(Some(friendly)));
    send_emote(&mut app, connection, EmoteKind::Hug, Some(enemy));
    advance(&mut app);
    let observed = &app.world().resource::<HugReceipts>().0;
    assert!(
        observed
            .iter()
            .any(|event| event.emote == EmoteKind::Hug && event.player_entity == player.to_bits()),
        "native Hug delivered"
    );
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(
        state.criteria_progress.get(&2379),
        Some(&1),
        "actual targeted native Hug retains all three qualifiers"
    );
    assert!(state.earned_ids.contains(&247));
    // Number of hugs1042 counts each accepted emote, even non-qualifying targets.
    assert_eq!(state.criteria_progress.get(&3231), Some(&11));
    let date = state.earned_at[&247];
    assert!(date > 0);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(enemy)
            .unwrap()
            .earned_ids
            .contains(&247)
    );
    advance(&mut app);
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&2379),
        Some(&1),
        "no idle replay"
    );
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert_eq!(saved.criteria_progress.get(&2379), Some(&1));
    assert!(saved.earned_ids.contains(&247));
    assert_eq!(saved.criteria_progress.get(&3231), Some(&11));
    assert_eq!(saved.earned_at[&247], date);
}
