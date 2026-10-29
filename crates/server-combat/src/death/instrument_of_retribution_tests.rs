use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::{
    ObservedTriggeredCasts, group_fixture_players, harness_with_plugins, spawn_npc, spawn_player,
};
use shared::components::{ActiveSpec, Health, Position};

#[test]
#[ignore = "requires canonical world.db"]
fn instrument_player_death_group_range_and_refresh() {
    let mut h = harness_with_plugins(vec![], &[404752, 31884, 5384], 2, false, |app| {
        app.add_plugins(crate::death::DeathPlugin);
    })
    .expect("world.db");
    let paladin = h.caster;
    h.app.init_resource::<ObservedTriggeredCasts>();
    h.app.world_mut().entity_mut(paladin).insert(ActiveSpec(70));
    let member = spawn_player(&mut h.app, 1, 40.0);
    group_fixture_players(&mut h.app, (paladin, 1), (member, 2));
    crate::combat::kill_unit(h.app.world_mut(), h.npc, member);
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(paladin)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 31884)
        .unwrap();
    assert_eq!(aura.duration_ms, 9000);

    let previous_expiry = aura.expires_at;
    let mut groups = h
        .app
        .world_mut()
        .resource_mut::<crate::group::GroupRegistry>();
    groups.convert_to_raid(1);
    assert!(groups.is_in_raid(1));
    drop(groups);
    let outsider = spawn_player(&mut h.app, 1, 1.0);
    h.app
        .world_mut()
        .entity_mut(outsider)
        .insert(crate::persistence::ClientId(3));
    crate::combat::kill_unit(h.app.world_mut(), h.npc, outsider);
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(paladin)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 31884)
        .unwrap();
    assert_eq!(aura.expires_at, previous_expiry);

    h.app.world_mut().get_mut::<Health>(member).unwrap().current = 1000.0;
    h.app
        .world_mut()
        .entity_mut(member)
        .remove::<crate::death::PlayerDeathState>();
    h.app.world_mut().get_mut::<Position>(member).unwrap().x = 40.1;
    crate::combat::kill_unit(h.app.world_mut(), h.npc, member);
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(paladin)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 31884)
        .unwrap();
    assert_eq!(aura.expires_at, previous_expiry);

    h.app.world_mut().get_mut::<Health>(member).unwrap().current = 1000.0;
    h.app
        .world_mut()
        .entity_mut(member)
        .remove::<crate::death::PlayerDeathState>();
    h.app.world_mut().get_mut::<Position>(member).unwrap().x = 0.0;
    crate::combat::kill_unit(h.app.world_mut(), h.npc, member);
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(paladin)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 31884)
        .unwrap();
    assert!(aura.expires_at.unwrap() > previous_expiry.unwrap());

    let expiry = aura.expires_at;
    h.app
        .world_mut()
        .entity_mut(paladin)
        .insert(KnownSpells::fixture(vec![31884]));
    h.app.world_mut().get_mut::<Health>(member).unwrap().current = 1000.0;
    h.app
        .world_mut()
        .entity_mut(member)
        .remove::<crate::death::PlayerDeathState>();
    crate::combat::kill_unit(h.app.world_mut(), h.npc, member);
    h.advance(2);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(paladin)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 31884)
        .unwrap();
    assert_eq!(aura.expires_at, expiry);

    h.app
        .world_mut()
        .entity_mut(paladin)
        .insert(KnownSpells::fixture(vec![404752, 31884]));
    h.app.world_mut().get_mut::<Health>(member).unwrap().current = 1000.0;
    h.app
        .world_mut()
        .entity_mut(member)
        .remove::<crate::death::PlayerDeathState>();
    let before_feign = h
        .app
        .world()
        .resource::<ObservedTriggeredCasts>()
        .0
        .iter()
        .filter(|(_, spell, _)| *spell == 31884)
        .count();
    crate::spell_cast::cast_triggered(h.app.world_mut(), member, 5384, Some(member), 1);
    h.advance(2);
    assert!(crate::auras::has_aura(h.app.world(), member, 5384));
    assert!(h.app.world().get::<Health>(member).unwrap().current > 0.0);
    assert_eq!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .iter()
            .filter(|(_, spell, _)| *spell == 31884)
            .count(),
        before_feign
    );
    let count = h.app.world().resource::<ObservedTriggeredCasts>().0.len();
    let creature = spawn_npc(&mut h.app, 1.0);
    h.app
        .world_mut()
        .entity_mut(creature)
        .insert(crate::hunter_pet::HunterPet {
            owner: member,
            slot: 0,
        });
    crate::combat::kill_unit(h.app.world_mut(), h.npc, creature);
    h.advance(2);
    assert_eq!(
        h.app.world().resource::<ObservedTriggeredCasts>().0.len(),
        count
    );
    crate::combat::kill_unit(h.app.world_mut(), h.npc, paladin);
    h.advance(2);
    assert_eq!(
        h.app.world().resource::<ObservedTriggeredCasts>().0.len(),
        count
    );
    crate::combat::kill_unit(h.app.world_mut(), h.npc, member);
    h.advance(2);
    assert_eq!(
        h.app.world().resource::<ObservedTriggeredCasts>().0.len(),
        count,
        "a dead paladin must not receive a proc when a group member dies"
    );
}
