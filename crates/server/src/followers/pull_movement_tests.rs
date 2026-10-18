//! Real kit AI → pursue → native fixed chase must retain one admitted movement point.
use super::pull_route_tests::route_fixture;
use super::tank_planner::AdmittedPosition;
use super::{Follower, catalog, kit};
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::Position;
use shared::protocol::CombatLogKind;
use std::collections::BTreeSet;

#[test]
fn tank_pull_movement_kit_pursuit_cannot_replace_the_admitted_position() {
    let (mut h, tank) = route_fixture();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = catalog::load_catalog(&db).unwrap();
    let kit_attacks: BTreeSet<_> = catalog
        .kits
        .iter()
        .filter(|entry| {
            entry.spec_id == 66 && matches!(entry.slot, kit::Slot::Attack | kit::Slot::Aoe)
        })
        .map(|entry| entry.spell_id)
        .collect();
    h.app.insert_resource(catalog);
    let owner = h.caster;
    let profile = h.app.world().get::<Follower>(tank).unwrap().profile.clone();
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    // Existing native Riverpaw floor oracle: inside Shield range, outside melee reach.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(128.5, 52.0, -33.86);
    let from = Position { x, y, z };
    h.app.world_mut().entity_mut(tank).insert(from);
    h.app.world_mut().entity_mut(owner).insert(from);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    let enemy = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == 20375682 && **copy == map)
        .unwrap()
        .0;
    h.attack(enemy);
    h.advance(3);
    let admitted = AdmittedPosition(Vec3::new(from.x, from.y, from.z));
    h.app.world_mut().entity_mut(tank).insert(admitted);
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let positions: Vec<_> = (0..90)
        .map(|_| {
            h.advance(1);
            *h.app.world().get::<Position>(tank).unwrap()
        })
        .collect();
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(tank.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && event
                    .spell_id
                    .is_some_and(|spell| kit_attacks.contains(&spell))),
        "real sourced kit attacks must remain available from the admitted point"
    );
    for (tick, at) in positions.iter().enumerate() {
        assert_eq!(
            Vec3::new(at.x, at.y, at.z),
            admitted.0,
            "kit pursuit overwrote the planner point at tick{tick}"
        );
    }
}
