use super::test_harness::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
fn achievements_criteria_kill_any_creature_uses_real_taps_group_range_and_corpse_eligibility() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use shared::components::{Npc, Position};
    use shared::death::{Corpse, DeathState};
    let mut h = harness(Vec::new(), &[], 1).expect("native creature damage pipeline");
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (killer, _) = persistence::create_character(&db, 1, "Mobkillproof", 1, 1).unwrap();
    assert_eq!(killer, 1);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(killer), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: killer,
    });
    let mut actors = vec![(killer, h.caster, "Mobkillproof")];
    for (index, name) in ["Alive", "Corpse", "Nocorpse", "Remote"]
        .into_iter()
        .enumerate()
    {
        let account = index as u64 + 2;
        let (character, _) = persistence::create_character(&db, account, name, 1, 1).unwrap();
        let player = spawn_player(&mut h.app, 1, 2.0);
        h.app
            .world_mut()
            .entity_mut(player)
            .insert((ClientId(character), PlayerAchievementState::default()));
        h.app.world_mut().spawn((
            Authenticated {
                account_id: account,
            },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
        ));
        if name == "Corpse" || name == "Nocorpse" {
            let corpse = (name == "Corpse").then_some(Corpse {
                player: player.to_bits(),
                map_id: 0,
                x: 2.0,
                y: 0.0,
                z: 0.0,
                created_at: 0,
            });
            h.app
                .world_mut()
                .entity_mut(player)
                .insert(crate::death::PlayerDeathState {
                    state: DeathState::Dead,
                    corpse,
                    graveyard: None,
                });
            h.app.world_mut().get_mut::<Health>(player).unwrap().current = 0.0;
        }
        if name == "Remote" {
            h.app.world_mut().entity_mut(player).insert(Position {
                x: 200.0,
                y: 0.0,
                z: 0.0,
            });
        }
        actors.push((character, player, name));
    }
    let online: OnlineView = actors
        .iter()
        .map(|&(character, player, name)| {
            (
                character,
                OnlineMember {
                    entity: player.to_bits(),
                    name: name.into(),
                    class: 1,
                    level: 80,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut group = GroupRegistry::default();
    group.sync_presence(&online);
    for &(character, _, name) in &actors[1..] {
        group.invite(killer, name, &online);
        group.respond_invite(character, true);
    }
    assert_eq!(group.group_members(killer).len(), 5);
    h.app.insert_resource(group);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let name: String = conn
        .query_row(
            "SELECT name FROM content_creature_template WHERE entry=299",
            [],
            |row| row.get(0),
        )
        .unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(Npc {
        template_id: 299,
        name: name.clone(),
    });
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100.0,
        max: 100.0,
    };
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase { db });
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    let finisher = spawn_npc(&mut h.app, 3.0);
    // Player's real nonlethal wound creates the tap; an unowned creature lands the lethal hit.
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.npc, 1.0);
    crate::combat::deal_direct_damage(h.app.world_mut(), finisher, h.npc, 200.0);
    assert_eq!(h.health(h.npc), 0.0);
    h.advance(3);
    for &(_, player, name) in &actors {
        let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
        let expected = matches!(name, "Mobkillproof" | "Alive" | "Nocorpse").then_some(1);
        assert_eq!(
            state.criteria_progress.get(&4944).copied(),
            expected,
            "{name}"
        );
        assert!(
            !state.criteria_progress.contains_key(&5529),
            "tap recipients did not land the blow"
        );
    }
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.npc, 200.0);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&4944),
        Some(&1)
    );
    let other = spawn_npc(&mut h.app, 5.0);
    h.app.world_mut().entity_mut(other).insert(Npc {
        template_id: 299,
        name,
    });
    // Moving the previously distant group member into another copy cannot make it eligible.
    let remote = actors[4].1;
    h.app.world_mut().entity_mut(remote).insert((
        Position {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
        crate::maps::InMap {
            map_id: 0,
            instance_id: 99,
        },
    ));
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, other, 1000.0);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&4944),
        Some(&2)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(remote)
            .unwrap()
            .criteria_progress
            .contains_key(&4944)
    );
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, killer)
            .criteria_progress
            .get(&4944),
        Some(&2)
    );
    assert!(
        !persistence::load_achievements(&reopened, actors[2].0)
            .criteria_progress
            .contains_key(&4944)
    );
}

#[test]
fn achievements_criteria_native_fireball_and_melee_each_credit_the_actual_hit_once() {
    let mut h = harness(Vec::new(), &[133], 8).expect("real Fireball and melee pipeline");
    let db = persistence::temp_database();
    let (character, _) = persistence::create_character(&db, 1, "Spellhit", 1, 8).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(character), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: character,
    });
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase { db });
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    h.cast_and_deliver(133, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&162),
        "launch is not damage"
    );
    h.advance(90);
    let spell_damage: u64 = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(133))
        .map(|event| event.amount as u64)
        .sum();
    assert!(spell_damage > 0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&162),
        Some(&spell_damage)
    );
    h.attack(h.npc);
    h.advance(55);
    h.stop_attack();
    h.advance(5);
    let melee_damage: u64 = h
        .inbox
        .log
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id.is_none())
        .map(|event| event.amount as u64)
        .sum();
    assert!(melee_damage > 0, "actual native swing lands");
    let total = spell_damage + melee_damage;
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&162),
        Some(&total)
    );
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&162),
        Some(&total)
    );
}

#[test]
fn achievements_criteria_native_damage_caps_totals_not_peaks_and_excludes_self_and_dead_replay() {
    let mut h = harness(Vec::new(), &[133], 8).expect("native damage world");
    let victim = spawn_player(&mut h.app, 1, 2.0);
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (caster_id, _) = persistence::create_character(&db, 1, "Hitproof", 1, 8).unwrap();
    let (victim_id, _) = persistence::create_character(&db, 2, "Hitvictim", 1, 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((ClientId(caster_id), PlayerAchievementState::default()));
    h.app
        .world_mut()
        .entity_mut(victim)
        .insert((ClientId(victim_id), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: caster_id,
    });
    h.app.world_mut().spawn((
        Authenticated { account_id: 2 },
        InWorld {
            character_id: victim_id,
        },
        PlayerLink(victim),
    ));
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase { db });
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    h.app.add_message::<crate::death::PlayerDeathTransition>();
    *h.app.world_mut().get_mut::<Health>(victim).unwrap() = Health {
        current: 100.0,
        max: 100.0,
    };
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, victim, 150.0);
    assert_eq!(h.health(victim), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5529),
        Some(&1)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&162),
        Some(&100)
    );
    let state = h.app.world().get::<PlayerAchievementState>(victim).unwrap();
    assert_eq!(state.criteria_progress.get(&5371), Some(&150));
    assert_eq!(state.criteria_progress.get(&5372), Some(&100));
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, victim, 500.0);
    crate::combat::deal_direct_damage(h.app.world_mut(), h.caster, h.caster, 20.0);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap();
    assert_eq!(state.criteria_progress.get(&162), Some(&100));
    assert_eq!(state.criteria_progress.get(&5371), Some(&20));
    assert!(!state.criteria_progress.contains_key(&5372));
    crate::combat::kill_unit(h.app.world_mut(), h.caster, h.caster);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5529),
        Some(&2)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .criteria_progress
            .get(&5371),
        Some(&20)
    );
    h.advance(5);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(victim)
            .unwrap()
            .criteria_progress
            .get(&5372),
        Some(&100)
    );
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, caster_id)
            .criteria_progress
            .get(&162),
        Some(&100)
    );
    assert_eq!(
        persistence::load_achievements(&reopened, caster_id)
            .criteria_progress
            .get(&5529),
        Some(&2)
    );
    let saved = persistence::load_achievements(&reopened, victim_id);
    assert_eq!(saved.criteria_progress.get(&5371), Some(&150));
    assert_eq!(saved.criteria_progress.get(&5372), Some(&100));
}
