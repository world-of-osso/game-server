use super::*;

#[test]
fn allied_default_summon_is_friendly_to_owner_and_party_and_excluded_from_aoe() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use crate::spell_cast::targets::{Relation, relation_matches};
    use crate::spell_cast::test_harness::{harness, spawn_player};
    let mut h =
        harness(Vec::new(), &[1308399, 5487, 77758, 192090], 11).expect("native world data");
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let owner = h.caster;
    let target = h.npc;
    let faction = *h.app.world().get::<UnitFactionTemplate>(owner).unwrap();
    let member = spawn_player(&mut h.app, 11, 2.0);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::persistence::ClientId(9001));
    h.app
        .world_mut()
        .entity_mut(member)
        .insert((crate::persistence::ClientId(9002), faction));
    let online: OnlineView = [(9001, "Owner"), (9002, "Party")]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                OnlineMember {
                    entity: id,
                    name: name.into(),
                    class: 11,
                    level: 90,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: Default::default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.invite(9001, "Party", &online);
    groups.respond_invite(9002, true);
    h.app.insert_resource(groups);
    crate::spell_cast::cast_triggered(h.app.world_mut(), owner, 1308399, Some(owner), 1);
    let world = h.app.world_mut();
    let summon = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 1308399)
        .map(|(entity, _)| entity)
        .expect("real Wild Guardian summon");
    assert!(relation_matches(world, owner, summon, Relation::Ally));
    assert!(relation_matches(world, member, summon, Relation::Raid));
    assert!(!relation_matches(world, owner, summon, Relation::Enemy));
    h.cast_and_deliver(5487, Some(owner));
    h.advance(40);
    h.cast_and_deliver(77758, Some(target));
    h.advance(40);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(owner.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(77758)
                && event.kind == shared::protocol::CombatLogKind::Damage),
        "Thrash must actually hit the enemy"
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(owner.to_bits())
                && event.target == Some(summon.to_bits())
                && event.kind == shared::protocol::CombatLogKind::Damage)
    );
    assert!(!crate::auras::has_aura(h.app.world(), summon, 192090));
}

#[test]
fn toyfx55_wild_initial_mana_uses_selected_level_and_requires_source_row() {
    let mut data = crate::world_data::load_world_data(&crate::test_data::world_db_path());
    let template = data.templates[&60].clone(); // Ruklar the Trapper: class2, ManaModifier1.
    assert_eq!(template.unit_class, 2);
    assert_eq!(template.mana_modifier, 1.0);
    assert_eq!(
        read_summon_mana(&data, &template, 20).unwrap().unwrap().max,
        490.0
    );
    assert_eq!(
        read_summon_mana(&data, &template, 30).unwrap().unwrap().max,
        852.0
    );
    data.class_level_base_mana.remove(&(30, 2));
    assert!(read_summon_mana(&data, &template, 30).is_none());
}

#[test]
fn guardian51_level_selection_preserves_flags_and_signed_delta() {
    use crate::world_data::summon_levels::GuardianLevelRange;
    let inputs = |range, delta| GuardianLevelRange {
        content_tuning_id: 0,
        range,
        delta,
    };
    assert_eq!(roll_guardian_level(20, inputs((1, 30), (0, 0)), 0).0, 20);
    assert_eq!(roll_guardian_level(80, inputs((1, 30), (0, 0)), 0).0, 30);
    assert_eq!(roll_guardian_level(1, inputs((5, 30), (2, 2)), 0).0, 7);
    assert_eq!(roll_guardian_level(80, inputs((5, 30), (-2, -2)), 0).0, 28);
    assert_eq!(
        roll_guardian_level(20, inputs((1, 30), (2, 2)), USE_CREATURE_LEVEL).0,
        32
    );
    assert_eq!(
        roll_guardian_level(20, inputs((1, 30), (0, 0)), ONLY_VISIBLE_TO_SUMMONER).0,
        20
    );
    assert_eq!(roll_guardian_level(80, inputs((0, 0), (0, 0)), 0).0, 0);
    assert_eq!(
        roll_guardian_level(80, inputs((0, 0), (0, 0)), USE_CREATURE_LEVEL).0,
        1
    );
    for _ in 0..64 {
        let level = roll_guardian_level(80, inputs((1, 30), (3, -2)), 0).0;
        assert!(
            (28..=33).contains(&level),
            "source sorted inclusive delta {level}"
        );
    }
}

#[test]
fn toyfx28_sourced_signed_title_sentinel_does_not_abort_other_summon_properties() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE summon_properties (ID INTEGER, Control INTEGER, Faction INTEGER, Title INTEGER, Slot INTEGER, Flags_0 INTEGER);
        INSERT INTO summon_properties VALUES (3001,1,0,-1,0,4610),(4493,1,0,2,0,67592);").unwrap();
    let properties = load_summon_properties(&conn).unwrap();
    assert_eq!(i64::from(properties.0[&3001].title), -1);
    // TC EffectSummonType checks JoinSummonerSpawnGroup before Title, even-1.
    assert!(properties.0[&3001].is_supported());
    assert!(properties.0[&3001].is_guardian());
    assert!(properties.0[&4493].is_supported());
    assert!(!properties.0[&4493].is_pet());
}
