//! TC a352b1fa spell_item.cpp3603–3641: real enemy death→one corpse flag.
use super::*;
use shared::components::{Health, Player, UnitAuras, UnitFactionTemplate};
use shared::protocol::GameObjectInfo;

fn flag_harness(path: &std::path::Path) -> (Harness, u64) {
    let (mut h, character) = setup(path);
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    bevy::app::Plugin::build(&crate::death::DeathPlugin, &mut h.app);
    h.app.init_resource::<ToyChatCapture>();
    h.app.add_systems(PostUpdate, capture_toy_chat);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ChatMessage>::default(),
        MessageReceiver::<ChatMessage>::default(),
    ));
    h.app
        .add_systems(FixedUpdate, crate::game_objects::summoned::expire_objects);
    learn(&mut h, 1);
    (h, character)
}

fn kill_flag_enemy(h: &mut Harness, x: f32, name: &str, race: u8) -> Entity {
    let db = &h.app.world().resource::<GameDatabase>().db;
    let (character, _) = persistence::create_character(db, 2, name, race, 1).unwrap();
    let enemy = spawn_player(&mut h.app, 1, x);
    h.app.world_mut().get_mut::<Player>(enemy).unwrap().name = name.into();
    let template = crate::faction::faction_template_for_race(race);
    h.app.world_mut().entity_mut(enemy).insert((
        UnitFactionTemplate(template),
        crate::persistence::ClientId(character),
    ));
    crate::combat::kill_unit(h.app.world_mut(), h.caster, enemy);
    h.advance(2);
    assert_eq!(h.app.world().get::<Health>(enemy).unwrap().current, 0.0);
    assert!(
        h.app
            .world()
            .get::<crate::death::PlayerDeathState>(enemy)
            .unwrap()
            .corpse
            .is_some()
    );
    enemy
}

#[test]
fn toy_coverage_flag51640_actual_enemy_death_plants_one_source_flag_and_expires() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = flag_harness(&dir.path().join("flag.redb"));
    let enemy = kill_flag_enemy(&mut h, 2.0, "Fallenorc", 2);
    let friendly = kill_flag_enemy(&mut h, 1.0, "Fallenally", 1);
    let far = kill_flag_enemy(&mut h, 30.0, "Farorc", 2);
    let wrong_map = kill_flag_enemy(&mut h, 2.5, "Othermaporc", 2);
    h.app
        .world_mut()
        .entity_mut(wrong_map)
        .insert(crate::maps::InMap::continent(1));
    crate::combat::kill_unit(h.app.world_mut(), h.caster, h.npc);
    h.advance(2);
    let body = *h.app.world().get::<Position>(enemy).unwrap();
    assert_eq!(activate(&mut h, FLAG_OWNERSHIP)[0].error, None);
    let world = h.app.world_mut();
    let objects: Vec<_> = world
        .query::<(Entity, &GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| info.entry == 190589)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(objects.len(), 1);
    let flag = objects[0];
    assert_eq!(world.get::<Position>(flag), Some(&body));
    assert_eq!(world.get::<GameObjectInfo>(flag).unwrap().display_id, 7953);
    assert!(
        world
            .get::<UnitAuras>(enemy)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 51657)
    );
    for other in [h.caster, friendly, far, wrong_map, h.npc] {
        assert!(
            !world
                .get::<UnitAuras>(other)
                .is_some_and(|auras| auras.auras.iter().any(|a| a.spell_id == 51657))
        );
    }
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == 51640 && go.hit_targets == [enemy.to_bits()])
    );
    // Full-mask child casts use the ordinary triggered path: prove their actual
    // object/aura effects above, not a separate child SpellGo packet.
    let emotes = &h.app.world().resource::<ToyChatCapture>().0[&h.connection];
    assert!(
        emotes.iter().any(|m| m.channel == ChatType::Emote
            && m.content.contains("Fallenorc")
            && m.content.contains("Flag of Ownership")),
        "{emotes:?}"
    );
    h.advance(1170);
    assert!(h.app.world().get_entity(flag).is_ok());
    h.advance(40);
    assert!(h.app.world().get_entity(flag).is_err());
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(enemy)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 51657)
    );
    assert_eq!(
        activate(&mut h, FLAG_OWNERSHIP)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toy_coverage_flag51640_no_enemy_body_refuses_before_effects_and_cooldown() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = flag_harness(&dir.path().join("no-body.redb"));
    kill_flag_enemy(&mut h, 2.0, "Onlyally", 1);
    assert_eq!(
        activate(&mut h, FLAG_OWNERSHIP)[0].error.as_deref(),
        Some("NoValidTargets")
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| [51640, 51657].contains(&go.spell_id))
    );
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 51640));
    assert!(
        h.app.world().resource::<toy_catalog::ToyCatalog>().0[&FLAG_OWNERSHIP]
            .snapshot
            .unavailable_reason
            .is_none()
    );
}

#[test]
fn toy_coverage_flag51640_two_enemy_bodies_selects_exactly_one() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = flag_harness(&dir.path().join("random-body.redb"));
    let first = kill_flag_enemy(&mut h, 2.0, "Firstorc", 2);
    let second = kill_flag_enemy(&mut h, 3.0, "Secondorc", 2);
    assert_eq!(activate(&mut h, FLAG_OWNERSHIP)[0].error, None);
    let go = h.inbox.go.iter().find(|go| go.spell_id == 51640).unwrap();
    assert_eq!(go.hit_targets.len(), 1);
    assert!([first.to_bits(), second.to_bits()].contains(&go.hit_targets[0]));
    let world = h.app.world_mut();
    let positions: Vec<_> = world
        .query::<(&GameObjectInfo, &Position)>()
        .iter(world)
        .filter(|(info, _)| info.entry == 190589)
        .map(|(_, position)| *position)
        .collect();
    assert_eq!(positions.len(), 1);
    let chosen = Entity::try_from_bits(go.hit_targets[0]).unwrap();
    assert_eq!(world.get::<Position>(chosen), Some(&positions[0]));
}

#[test]
fn toy_coverage_flag51640_changed_source_root_is_not_admitted_as_a_generic_dummy() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = flag_harness(&dir.path().join("changed-root.redb"));
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let mut root = store.get(51640).unwrap().clone();
    let child = store.get(51657).unwrap().clone();
    root.effects[0].implicit_targets = [1, 0];
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(vec![root, child]));
    let result = activate(&mut h, FLAG_OWNERSHIP);
    assert!(
        result[0]
            .error
            .as_deref()
            .unwrap()
            .contains("source signature differs")
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == 51640));
    assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 51640));
}
