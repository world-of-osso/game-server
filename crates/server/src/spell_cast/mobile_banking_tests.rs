use super::test_harness::*;
use bevy::prelude::*;
use shared::components::GuildMembership;
use shared::protocol::GameObjectInfo;

const MOBILE: u32 = 83958;

fn fixture(race: u8, member: bool) -> Harness {
    let mut h = harness(Vec::new(), &[MOBILE, 88304, 88306], 8).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    h.app
        .add_systems(Update, crate::game_objects::summoned::expire_objects);
    h.app
        .world_mut()
        .get_mut::<shared::components::Player>(h.caster)
        .unwrap()
        .race = race;
    let mut known = crate::class_progression::KnownSpells::fixture(Vec::new());
    if member {
        known.set_guild_grants(&[MOBILE]);
    }
    h.app.world_mut().entity_mut(h.caster).insert((
        known,
        shared::components::CombatStatus(false),
        shared::reputation::CharacterReputation::new_for_race(
            &shared::reputation::default_faction_registry(),
            race,
        ),
    ));
    if member {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(GuildMembership {
                guild_id: 1,
                guild_name: "Mobile Guild".into(),
            });
    }
    h.advance(4);
    h
}

fn chests(h: &mut Harness) -> Vec<(Entity, u32)> {
    let world = h.app.world_mut();
    world
        .query::<(Entity, &GameObjectInfo)>()
        .iter(world)
        .filter(|(_, info)| info.go_type == 34)
        .map(|(e, info)| (e, info.entry))
        .collect()
}

fn cast_chest(h: &mut Harness) -> Entity {
    h.inbox.failed.clear();
    h.cast(MOBILE, None);
    h.advance(75); // Installed CastingTimeIndex14: three seconds, plus delivery.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let objects = chests(h);
    assert_eq!(
        objects.len(),
        1,
        "A real perk cast must summon a guild chest"
    );
    objects[0].0
}

#[test]
fn guild_mobile_banking_neutral_member_casts_both_faction_chests_with_five_minute_lifetime() {
    for (race, entry) in [(1, 206602), (2, 206603)] {
        let mut h = fixture(race, true);
        let chest = cast_chest(&mut h);
        assert_eq!(chests(&mut h)[0].1, entry);
        assert_eq!(
            h.app
                .world()
                .get::<shared::protocol::GameObjectCollision>(chest),
            Some(&shared::protocol::GameObjectCollision(true))
        );
        let source = h
            .app
            .world()
            .get::<crate::game_objects::summoned::SummonedObject>(chest)
            .unwrap();
        let remaining = source.expires_at.unwrap() - super::now(h.app.world());
        assert!((299.0..=300.0).contains(&remaining), "{remaining}");
        h.app
            .world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(std::time::Duration::from_secs(7200));
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs(299),
            ));
        h.advance(1);
        assert!(h.app.world().get_entity(chest).is_ok());
        h.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs(1),
            ));
        h.advance(1);
        assert!(h.app.world().get_entity(chest).is_err());
    }
}

#[test]
fn guild_mobile_banking_user_policy_cooldown_refuses_at_one_hour_and_until_two_hours() {
    let mut h = fixture(1, true);
    let _ = cast_chest(&mut h);
    let update = h
        .inbox
        .cooldowns
        .iter()
        .find(|c| c.spell_id == MOBILE)
        .unwrap();
    assert_eq!(
        update.duration_ms, 7_200_000,
        "MOBILE_BANKING_COOLDOWN_2H overrides installed3600000"
    );
    let deadline = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .spell_ready_at[&MOBILE];
    for offset in [3600.0, 0.1] {
        let clock = h.app.world().resource::<Time<Real>>().elapsed_secs_f64();
        h.app.world_mut().resource_mut::<Time<Real>>().advance_by(
            std::time::Duration::from_secs_f64(deadline - offset - clock),
        );
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(MOBILE)
            .unwrap()
            .clone();
        assert!(matches!(
            super::start_cast(h.app.world_mut(), h.caster, &spell, None),
            Err(super::CastFailure::OnCooldown)
        ));
    }
    h.app
        .world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(std::time::Duration::from_millis(100));
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(MOBILE)
        .unwrap()
        .clone();
    assert!(super::start_cast(h.app.world_mut(), h.caster, &spell, None).is_ok());
}

#[test]
fn guild_mobile_banking_nonmember_cannot_cast_even_with_stale_known_spell() {
    let mut h = fixture(1, false);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![MOBILE]));
    h.cast(MOBILE, None);
    h.advance(75);
    assert!(
        !h.inbox.failed.is_empty(),
        "Nonmember stale knowledge must not summon"
    );
    assert!(chests(&mut h).is_empty());
}
