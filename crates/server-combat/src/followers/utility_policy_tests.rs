//! FOLLOWER_UTILITY_POLICY: real selected spellbooks and ordinary execution.
use super::{
    kit_tests::level32, profile_behavior_tests::spawn_profile_fixture_with_plugins, update_ai,
};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, Level, Npc, Position};
use shared::protocol::CombatLogKind;

pub(super) fn add_follower(h: &mut Harness, entry: u32) -> Entity {
    let catalog = h.app.world().resource::<super::FollowerCatalog>();
    let profile = catalog
        .profiles
        .iter()
        .find(|p| p.entry == entry)
        .unwrap()
        .clone();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[entry]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert(Npc {
        template_id: entry,
        name: profile.name.clone(),
    });
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    unit
}

#[test]
fn follower_utility_policy_native_resurrection_effect_restores_a_dead_follower() {
    let (mut h, unit) = level32(209072);
    let healer = add_follower(&mut h, 215011);
    h.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::combat::CorpseDelay(60.0));
    h.app.world_mut().get_mut::<Position>(healer).unwrap().x = 3.0;
    let corpse_position = *h.app.world().get::<Position>(healer).unwrap();
    crate::combat::kill_unit(h.app.world_mut(), h.npc, healer);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(healer)
            .is_some()
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 20484, Some(healer), false).unwrap();
    h.advance(80);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(20484)
        .unwrap();
    let percent = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 18)
        .unwrap()
        .base_points;
    let max = h.app.world().get::<Health>(healer).unwrap().max;
    assert!(h.health(healer) >= (max * percent / 100.0).floor());
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(healer)
            .is_none()
    );
    assert!(h.app.world().get::<crate::combat::Corpse>(healer).is_none());
    assert_eq!(
        *h.app.world().get::<Position>(healer).unwrap(),
        corpse_position
    );
}

#[test]
fn follower_native_mass_resurrection_restores_only_its_raid_corpses() {
    use crate::group::{GroupRegistry, OnlineMember, OnlineView};
    use shared::group::GroupRole;

    let (mut h, caster) = super::kit_tests::at_level_unengaged(215011, 90, |_| {});
    let target = add_follower(&mut h, 209072);
    let outsider = add_follower(&mut h, 209059);
    let fillers = [add_follower(&mut h, 209059), add_follower(&mut h, 209059)];
    let filler_ids = fillers.map(super::Follower::member_id);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(1));
    let caster_id = super::Follower::member_id(caster);
    let target_id = super::Follower::member_id(target);
    let online: OnlineView = [
        (1, h.caster),
        (caster_id, caster),
        (target_id, target),
        (filler_ids[0], fillers[0]),
        (filler_ids[1], fillers[1]),
    ]
    .into_iter()
    .map(|(id, unit)| {
        (
            id,
            OnlineMember {
                entity: unit.to_bits(),
                name: format!("Member{id}"),
                class: 5,
                level: 90,
                portrait: Default::default(),
                dungeon_difficulty: 0,
                raid_difficulty: Default::default(),
            },
        )
    })
    .collect();
    h.app
        .world_mut()
        .resource_mut::<GroupRegistry>()
        .create_companion_party(
            1,
            GroupRole::Dps,
            &[
                (caster_id, GroupRole::Healer),
                (target_id, GroupRole::Tank),
                (filler_ids[0], GroupRole::Dps),
                (filler_ids[1], GroupRole::Dps),
            ],
            &online,
        )
        .unwrap();
    for unit in [target, outsider] {
        h.app.world_mut().get_mut::<Position>(unit).unwrap().x = 3.0;
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(crate::combat::CorpseDelay(60.0));
        crate::combat::kill_unit(h.app.world_mut(), h.npc, unit);
    }
    let body = *h.app.world().get::<Position>(target).unwrap();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(212036)
        .unwrap()
        .clone();
    crate::spell_cast::execute_cast(h.app.world_mut(), caster, &spell, None).unwrap();
    assert!(
        h.health(target) > 0.0,
        "native Mass Resurrection must reach a follower corpse"
    );
    assert!(h.app.world().get::<crate::combat::Corpse>(target).is_none());
    assert_eq!(*h.app.world().get::<Position>(target).unwrap(), body);
    assert_eq!(
        h.health(outsider),
        0.0,
        "an unrelated follower must remain dead"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::Corpse>(outsider)
            .is_some()
    );
}

#[test]
fn follower_utility_policy_native_resurrection_from_follower_retains_player_acceptance() {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::protocol::{ResurrectionOffer, ResurrectionResponse};
    let (mut h, unit) = level32(209072);
    h.stop_attack();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Dead,
            corpse: None,
            graveyard: None,
        });
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ResurrectionOffer>::default(),
        MessageReceiver::<ResurrectionOffer>::default(),
    ));
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 20484, Some(h.caster), false)
        .unwrap();
    let mut offers = Vec::new();
    for _ in 0..80 {
        h.advance(1);
        offers.extend(crate::spell_cast::test_harness::drain::<ResurrectionOffer>(
            &mut h.app,
            h.connection,
        ));
        if !offers.is_empty() {
            break;
        }
    }
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].caster_name, "Crenna Earth-Daughter");
    assert_eq!(h.health(h.caster), 0.0, "player must not auto-accept");
    crate::death::resurrection::respond(
        h.app.world_mut(),
        h.caster,
        ResurrectionResponse {
            caster: unit.to_bits(),
            spell_id: 20484,
            accept: true,
        },
    );
    assert!(h.health(h.caster) > 0.0);
}

#[test]
fn follower_headless_owner_accepts_native_resurrection_offer() {
    let (mut h, unit) = level32(209072);
    h.stop_attack();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Dead,
            corpse: None,
            graveyard: None,
        });
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    crate::test_support::enable_headless_resurrection(h.app.world_mut(), h.caster);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 20484, Some(h.caster), false)
        .unwrap();
    h.advance(80);
    assert!(crate::test_support::accept_resurrection(
        h.app.world_mut(),
        h.caster
    ));
    assert!(h.health(h.caster) > 0.0);
}

#[test]
fn follower_utility_policy_no_damage_into_someone_elses_cc() {
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(209059, |_| {});
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 32;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(118)
        .unwrap();
    let mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: 118,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(h.caster)));
    // Only follower behavior is under test; the owner must not white-swing the sheep.
    h.stop_attack();
    h.advance(3);
    let before = h.health(h.npc);
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(h.health(h.npc), before);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 118));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.source == Some(unit.to_bits()))
    );
}
