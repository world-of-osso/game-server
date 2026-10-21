//! TC a352b1fa Spell.cpp1393–1401: target118 expands only within the caster's group.
use bevy::prelude::*;
use shared::components::{Npc, Player, Position};
use shared::group::GroupRole;

use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;
use crate::spell_cast::test_harness::{Harness, harness, spawn_player};

const FORTITUDE: u32 = 21_562;

fn online(unit: Entity, name: String, class: u8) -> OnlineMember {
    OnlineMember {
        entity: unit.to_bits(),
        name,
        class,
        level: 80,
        portrait: default(),
        dungeon_difficulty: 1,
        raid_difficulty: default(),
    }
}

fn player_party() -> (Harness, Vec<Entity>, Entity) {
    let mut h = harness(Vec::new(), &[FORTITUDE], 5).unwrap();
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(ClientId(1));
    let mut members = vec![caster];
    // Native radius100 is centered on targeted member x30, not on the caster.
    for (id, x) in [(2, 30.0), (3, 120.0), (4, 20.0), (5, 10.0)] {
        let unit = spawn_player(&mut h.app, 5, x);
        h.app.world_mut().entity_mut(unit).insert(ClientId(id));
        members.push(unit);
    }
    let view: OnlineView = members
        .iter()
        .enumerate()
        .map(|(i, &unit)| {
            let id = i as u64 + 1;
            (id, online(unit, format!("Buff{id}"), 5))
        })
        .collect();
    let mut groups = GroupRegistry::default();
    for id in 2..=5 {
        groups.invite(1, &format!("Buff{id}"), &view);
        groups.respond_invite(id, true);
        assert!(groups.is_in_same_group(1, id));
    }
    h.app.insert_resource(groups);
    let outsider = spawn_player(&mut h.app, 5, 31.0);
    h.app.world_mut().entity_mut(outsider).insert(ClientId(6));
    (h, members, outsider)
}

fn buffed(h: &Harness, unit: Entity) -> bool {
    crate::auras::has_aura(h.app.world(), unit, FORTITUDE)
}

#[test]
#[ignore = "RED target118 group expansion"]
fn census_tail_118_native_fortitude_buffs_all_five_around_target_not_caster() {
    let (mut h, members, outsider) = player_party();
    h.cast_and_deliver(FORTITUDE, Some(members[1]));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for unit in members {
        assert!(
            buffed(&h, unit),
            "party member {unit:?} must receive native Fortitude"
        );
    }
    assert!(!buffed(&h, outsider));
}

#[test]
fn census_tail_118_native_fortitude_ungrouped_target_does_not_buff_casters_party() {
    let (mut h, members, outsider) = player_party();
    h.cast_and_deliver(FORTITUDE, Some(outsider));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(buffed(&h, outsider));
    for unit in members {
        assert!(!buffed(&h, unit));
    }
}

#[test]
#[ignore = "RED target118 group expansion"]
fn census_tail_118_native_fortitude_excludes_far_dead_and_other_copy_members() {
    let (mut h, members, outsider) = player_party();
    h.app.world_mut().get_mut::<Position>(members[2]).unwrap().x = 131.0;
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(members[3])
        .unwrap()
        .current = 0.0;
    h.app
        .world_mut()
        .entity_mut(members[4])
        .insert(crate::maps::InMap {
            map_id: 0,
            instance_id: 77,
        });
    h.cast_and_deliver(FORTITUDE, Some(members[1]));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(buffed(&h, members[0]));
    assert!(buffed(&h, members[1]));
    for unit in members[2..].iter().copied().chain([outsider]) {
        assert!(!buffed(&h, unit));
    }
}

#[test]
#[ignore = "RED target118 follower group identity"]
fn census_tail_118_native_henry_fortitude_buffs_actual_companion_party() {
    let mut h = harness(Vec::new(), &[FORTITUDE], 5).unwrap();
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert(ClientId(1));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = super::catalog::load_catalog(&conn).unwrap();
    let mut view = OnlineView::from([(1, online(owner, "Owner".into(), 5))]);
    let mut companions = Vec::new();
    let mut units = vec![owner];
    let mut henry = None;
    let mut outsider = None;
    for entry in [209_057, 215_011, 209_053, 209_059, 209_065] {
        let profile = catalog
            .profiles
            .iter()
            .find(|profile| profile.entry == entry)
            .unwrap()
            .clone();
        let unit = spawn_player(&mut h.app, profile.class_id, 10.0);
        let follower = super::Follower { owner, profile };
        h.app
            .world_mut()
            .entity_mut(unit)
            .remove::<Player>()
            .insert((
                Npc {
                    template_id: entry,
                    name: follower.profile.name.clone(),
                },
                follower.clone(),
                shared::components::UnitSummonedBy(owner.to_bits()),
            ));
        let member = super::Follower::member_id(unit);
        if entry == 209_065 {
            outsider = Some(unit);
        } else {
            view.insert(member, follower.online(unit, 80, 1));
            companions.push((member, follower.profile.role));
            units.push(unit);
        }
        if entry == 215_011 {
            henry = Some(unit);
        }
    }
    let mut groups = GroupRegistry::default();
    groups
        .create_companion_party(1, GroupRole::Dps, &companions, &view)
        .unwrap();
    h.app.insert_resource(groups);
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        henry.unwrap(),
        FORTITUDE,
        Some(owner),
        false,
    )
    .unwrap();
    h.advance(3);
    for unit in units {
        assert!(
            buffed(&h, unit),
            "companion party member {unit:?} lacks native Fortitude"
        );
    }
    assert!(
        !buffed(&h, outsider.unwrap()),
        "ownership alone does not confer follower group membership"
    );
}
