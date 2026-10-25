use super::*;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins, spawn_npc};
use rusqlite::Connection;
use shared::components::{Health, Npc};

fn progress(h: &Harness, criterion: u32) -> Option<u64> {
    h.app
        .world()
        .get::<PlayerAchievementState>(h.caster)
        .unwrap()
        .criteria_progress
        .get(&criterion)
        .copied()
}

fn cast_mark(h: &mut Harness, spell: u32, receiver: Entity) {
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        h.npc,
        spell,
        Some(receiver),
        0,
    ));
    h.advance(2);
    assert!(
        crate::auras::has_aura(h.app.world(), receiver, spell),
        "native mark {spell}"
    );
}

/// Controlled boss identity/health, not a source boss spawn or encounter script.
fn spawn_controlled_target(h: &mut Harness, entry: u32) -> Entity {
    let target = spawn_npc(&mut h.app, 3.0);
    h.app.world_mut().entity_mut(target).insert((
        Npc {
            template_id: entry,
            name: "Controlled criteria victim".into(),
        },
        Health {
            current: 1.0,
            max: 1.0,
        },
    ));
    target
}

fn kill_with_shadow_bolt(h: &mut Harness, target: Entity) {
    h.cast(686, Some(target));
    h.advance(160);
    assert_eq!(h.health(target), 0.0, "genuine lethal spell damage");
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(target)
            .is_some()
    );
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == 686));
}

#[test]
fn achievements_criteria_absent_aura_native_marks_and_kills_persist() {
    // Marks are genuine source spell casts; identities/HP are controlled because
    // the installed world has neither boss template. No encounter-clear claim.
    for (entry, criterion, achievement, mark) in [
        (58739, 20693, 7257, None),
        (58739, 20693, 7257, Some(115013)),
        (96028, 29405, 10458, None),
        (96028, 29405, 10458, Some(196665)),
        (96028, 29405, 10458, Some(196666)),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Auraabsence", 1, 9).unwrap();
        let (foreign, _) = persistence::create_character(&db, 2, "Foreignaura", 1, 9).unwrap();
        let mut h = harness_with_plugins(vec![], &[686, 115013, 196665, 196666], 9, false, |app| {
            app.add_plugins(AchievementPlugin);
        })
        .unwrap();
        h.app.insert_resource(GameDatabase::new(db));
        h.app
            .insert_resource(crate::test_data::achievement_catalog());
        h.app.world_mut().entity_mut(h.caster).insert((
            persistence::ClientId(character),
            PlayerAchievementState::default(),
            Health {
                current: 1_000_000.0,
                max: 1_000_000.0,
            },
        ));
        h.app
            .world_mut()
            .get_mut::<InWorld>(h.connection)
            .unwrap()
            .character_id = character;
        if let Some(spell) = mark {
            let player = h.caster;
            cast_mark(&mut h, spell, player);
        }
        let target = spawn_controlled_target(&mut h, entry);
        assert!(!crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            h.caster,
            686,
            Some(target),
            u8::MAX
        ));
        h.advance(3);
        assert_eq!(progress(&h, criterion), None, "refused cast/living victim");
        kill_with_shadow_bolt(&mut h, target);
        let expected = mark.is_none().then_some(1);
        assert_eq!(
            progress(&h, criterion),
            expected,
            "entry {entry}, actor mark {mark:?}"
        );
        let earned = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .earned_ids
            .contains(&achievement);
        assert!(
            !earned,
            "ordinary kills cannot supply the required completed-map grant boundary"
        );
        if let Some(spell) = mark {
            // Native aura removal must permit a later distinct kill, not replay the old one.
            let player = h.caster;
            assert!(
                !crate::auras::remove_auras(
                    h.app.world_mut(),
                    player,
                    crate::auras::RemoveReason::Expired,
                    |aura| aura.spell_id == spell
                )
                .is_empty()
            );
            assert!(!crate::auras::has_aura(h.app.world(), player, spell));
            let next = spawn_controlled_target(&mut h, entry);
            // Controlled victim aura avoids Swamp Smash's lethal damage on the 1HP fixture.
            // Actor marks above use genuine casts; this control isolates aura ownership only.
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.npc,
                    target: next,
                    spell_id: spell,
                    effect_mask: 1 << 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            assert!(crate::auras::has_aura(h.app.world(), next, spell));
            kill_with_shadow_bolt(&mut h, next);
            assert_eq!(
                progress(&h, criterion),
                Some(1),
                "removed actor mark; victim mark irrelevant"
            );
        }
        let before_idle = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap()
            .clone();
        h.cast(686, Some(target));
        h.advance(160);
        let after_idle = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.caster)
            .unwrap();
        assert_eq!(after_idle.criteria_progress, before_idle.criteria_progress);
        assert_eq!(after_idle.earned_at, before_idle.earned_at);
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
        assert_eq!(saved.earned_at, before_idle.earned_at);
        assert!(
            persistence::load_achievements(&reopened, foreign)
                .criteria_progress
                .is_empty()
        );
    }
}

#[test]
fn achievements_criteria_absent_aura_authority_alive_and_difficulty() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut world = World::new();
    let actor = world
        .spawn((
            Player {
                name: "Authority".into(),
                race: 1,
                class: 9,
                appearance: Default::default(),
            },
            Level(10),
            InMap::continent(0),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let event = CriteriaEvent::KillCreature(58739);
    let mut state = PlayerAchievementState::default();
    let conditions = player_conditions(&world, actor);
    catalog.update_at(&mut state, event, None, 1, &conditions);
    assert_eq!(
        state.criteria_progress.get(&20693),
        None,
        "absent component is not known absence"
    );
    world
        .entity_mut(actor)
        .insert(crate::auras::ActiveAuras::default());
    world.get_mut::<Health>(actor).unwrap().current = 0.0;
    let conditions = player_conditions(&world, actor);
    catalog.update_at(&mut state, event, None, 1, &conditions);
    assert_eq!(
        state.criteria_progress.get(&20693),
        None,
        "alive16 companion retained"
    );
    world.get_mut::<Health>(actor).unwrap().current = 100.0;
    let conditions = player_conditions(&world, actor);
    catalog.update_at(&mut state, event, None, 2, &conditions);
    assert_eq!(state.criteria_progress.get(&20693), Some(&1));
    assert!(
        !state.earned_ids.contains(&7257),
        "map completion remains required"
    );
    catalog.update_at(&mut state, event, Some(1005), 2, &conditions);
    assert!(
        state.earned_ids.contains(&7257),
        "catalog accepts the authored map boundary"
    );
    for (difficulty, expected) in [(1, None), (2, None), (23, Some(1))] {
        let mut state = PlayerAchievementState::default();
        let mut conditions = player_conditions(&world, actor);
        conditions.difficulty = difficulty;
        catalog.update_at(
            &mut state,
            CriteriaEvent::KillCreature(96028),
            None,
            3,
            &conditions,
        );
        assert_eq!(
            state.criteria_progress.get(&29405).copied(),
            expected,
            "difficulty {difficulty}"
        );
    }
}
