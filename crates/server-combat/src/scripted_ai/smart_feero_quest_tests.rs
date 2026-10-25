//! Feero's authored three-wave credit and stored-invoker death failure.
use super::*;
use crate::persistence::quest_store::{
    ActiveQuestData, QuestLogData, load_quest_log, save_quest_log,
};

fn install_feero_quest(temp: &tempfile::TempDir, world: &mut World, player: Entity) {
    let db = redb::Database::create(temp.path().join("feero.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let quests = crate::quest_data::test_quest_db().unwrap().clone();
    let quest = &quests.quests[&976];
    assert!(
        quest
            .objectives
            .iter()
            .any(|o| o.kind == crate::quest_data::ObjectiveKind::ExplorationOrEvent)
    );
    save_quest_log(
        &db,
        1,
        &QuestLogData {
            active: vec![ActiveQuestData {
                quest_id: 976,
                counts: vec![0; quest.objectives.len()],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    world.insert_resource(crate::persistence::GameDatabase::new(db));
    world.insert_resource(quests);
    world.insert_resource(crate::quest::QuestDirty::default());
    world
        .entity_mut(player)
        .insert(crate::persistence::ClientId(1));
}

fn kill_wave(world: &mut World, player: Entity, entries: &[u32], expected: usize) {
    let ambushers: Vec<_> = world
        .query::<(Entity, &Npc, &Health)>()
        .iter(world)
        .filter(|(_, npc, health)| entries.contains(&npc.template_id) && health.current > 0.0)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(ambushers.len(), expected);
    for ambusher in ambushers {
        let ai = creature_ai(world, ambusher).unwrap();
        world.entity_mut(ambusher).insert(CreatureAiState {
            ai: Some(ai),
            pending: Default::default(),
        });
        world.get_mut::<Health>(ambusher).unwrap().current = 0.0;
        dispatch(world, ambusher, Hook::JustDied(Some(player)));
    }
}

fn assert_quest(world: &World, completed: bool, failed: bool) {
    let log = load_quest_log(&world.resource::<crate::persistence::GameDatabase>().db, 1);
    let active = log.active(976).unwrap();
    assert_eq!(active.failed, failed);
    let quests = world.resource::<crate::quest_data::QuestDb>();
    assert_eq!(
        crate::quest_rules::is_complete_in_log(quests, &log, 976, |_| 0),
        completed
    );
}

#[test]
fn smartai_feero_three_authored_waves_credit_only_after_final_deaths() {
    let (temp, mut world, creature, player) = feero();
    install_feero_quest(&temp, &mut world, player);
    for (start, end, entries, count, delays) in [
        (1, 19, vec![3879], 4, vec![0, 1000, 11000, 50]),
        (20, 27, vec![3893], 3, vec![0, 200, 1000, 7000, 50]),
        (28, 43, vec![3898, 3899, 3900], 3, vec![0, 1000, 3000, 50]),
    ] {
        for node in start..=end {
            reach(&mut world, creature, player, node);
        }
        assert_quest(&world, false, false);
        kill_wave(&mut world, player, &entries, count);
        for diff in delays {
            dispatch(&mut world, creature, Hook::UpdateAi(diff));
        }
    }
    assert_quest(&world, true, false);
    for node in 44..=48 {
        reach(&mut world, creature, player, node);
    }
    assert_quest(&world, true, false);
}

#[test]
fn smartai_feero_death_fails_stored_acceptor_before_credit() {
    let (temp, mut world, creature, player) = feero();
    install_feero_quest(&temp, &mut world, player);
    world.get_mut::<Health>(creature).unwrap().current = 0.0;
    dispatch(&mut world, creature, Hook::JustDied(None));
    assert_quest(&world, false, true);
}
