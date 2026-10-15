//! TC Unit::SetMinion: only a player owner sets PLAYER_CONTROLLED; that is not IsPet.
use super::*;

#[test]
fn toyfx42_npc_owned_guardian_is_not_player_controlled() {
    assert_source_actor_flags(0, 0x400202, false, false);
}

#[test]
fn toyfx42_player_companion_preserves_control_and_immunity_flags() {
    assert_source_actor_flags(5, 0, true, true);
}

#[test]
fn toyfx42_npc_companion_preserves_immunity_without_player_control() {
    assert_source_actor_flags(5, 0, false, false);
}

#[test]
fn toyfx42_plain_ally_is_owned_but_not_a_player_controlled_minion() {
    assert_source_actor_flags(0, 0, true, false);
}

/// Use the real Scarab spell/template but vary only title/flags and owner identity
/// to isolate minion ownership. Modified properties are not Scarab source claims.
fn assert_source_actor_flags(title: i32, flags: u32, owner_is_player: bool, controlled: bool) {
    use crate::spell_cast::summon::SummonedCreature;
    use shared::components::{Npc, Player, UnitFlags, UnitIsPet, UnitSummonedBy};
    let directory = tempfile::tempdir().unwrap();
    let path = toyfx8_world();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let store = crate::spell_info::load_spell_data(&conn).unwrap().0;
    let spell = store.get(92146).unwrap().clone();
    let (mut h, _) = setup(&directory.path().join("owner-flags.redb"));
    h.app.insert_resource(store);
    h.app.insert_resource(shared_world_data(&path));
    let mut properties = crate::spell_cast::summon::load_summon_properties(&conn).unwrap();
    let property = properties.0.get_mut(&2301).unwrap();
    property.title = title;
    property.flags = flags;
    h.app.insert_resource(properties);
    if !owner_is_player {
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .remove::<Player>()
            .insert(Npc {
                template_id: 6,
                name: "NPC owner fixture".into(),
            });
    }
    let caster = h.caster;
    crate::spell_cast::execute_cast(h.app.world_mut(), caster, &spell, None)
        .expect("source E28 cast");
    let world = h.app.world_mut();
    let actors: Vec<_> = world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 51113 && summon.spell_id == 92146)
        .map(|(entity, _, _)| entity)
        .collect();
    assert!(!actors.is_empty());
    for actor in actors {
        let actor_flags = world.get::<UnitFlags>(actor);
        assert_eq!(
            actor_flags
                .is_some_and(|flags| flags.0 & crate::faction::UNIT_FLAG_PLAYER_CONTROLLED != 0),
            controlled,
            "title{title}/flags{flags:#x}/player-owner{owner_is_player}"
        );
        if title == 5 {
            let immunity = 0x100 | 0x200; // Native IMMUNE_TO_PC / IMMUNE_TO_NPC.
            assert!(actor_flags.is_some_and(|flags| flags.0 & immunity == immunity));
        }
        assert_eq!(
            world.get::<UnitSummonedBy>(actor).unwrap().0,
            caster.to_bits()
        );
        assert!(!world.get::<UnitIsPet>(actor).is_some_and(|pet| pet.0));
    }
}
