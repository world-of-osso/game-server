//! Real-DB tests against data/world.db.

use rusqlite::{Connection, OpenFlags};

use super::*;

const STORMWIND: u32 = 2;
const SENTINEL_HILL: u32 = 4;
const LAKESHIRE: u32 = 5;
const MOONBROOK: u32 = 583;
/// Dungar Longdrink's spawn (creature guid 79658), WoW coordinates.
const DUNGAR: Vec3 = Vec3::new(-8835.76, 490.084, 109.699);

fn network() -> Option<TaxiNetwork> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(load_taxi_network(&conn).unwrap())
}

fn known(nodes: &[u32]) -> BTreeSet<u32> {
    nodes.iter().copied().collect()
}

#[test]
fn dungar_longdrink_stands_at_the_stormwind_node() {
    let Some(network) = network() else { return };
    assert_eq!(
        network.nearest_node(0, DUNGAR, Team::Alliance),
        Some(STORMWIND)
    );
    assert_eq!(network.nodes[&STORMWIND].name, "Stormwind, Elwynn");
}

#[test]
fn stormwind_to_sentinel_hill_is_one_hop_for_5_copper() {
    let Some(network) = network() else { return };
    let route = network
        .route(
            STORMWIND,
            SENTINEL_HILL,
            &known(&[STORMWIND, SENTINEL_HILL]),
            Team::Alliance,
        )
        .unwrap();
    assert_eq!(route.nodes, [STORMWIND, SENTINEL_HILL]);
    assert_eq!(route.cost, 5);
    let points = network.flight_points(&route.nodes);
    assert_eq!(points.len(), 28, "TaxiPath 6 has 28 points");
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-10550.8, 1034.49, 59.07);
    assert!(points.last().unwrap().distance(Vec3::new(x, y, z)) < 0.1);
}

#[test]
fn routes_only_pass_through_known_nodes() {
    let Some(network) = network() else { return };
    let route = network
        .route(
            STORMWIND,
            MOONBROOK,
            &known(&[STORMWIND, SENTINEL_HILL, MOONBROOK]),
            Team::Alliance,
        )
        .unwrap();
    assert_eq!(route.nodes, [STORMWIND, SENTINEL_HILL, MOONBROOK]);
    assert_eq!(route.cost, 10);
    assert!(
        network
            .route(
                STORMWIND,
                MOONBROOK,
                &known(&[STORMWIND, MOONBROOK]),
                Team::Alliance
            )
            .is_none(),
        "no direct path and Sentinel Hill unknown"
    );
}

#[test]
fn nodes_behind_a_player_condition_are_unreachable() {
    let Some(network) = network() else { return };
    assert!(
        !network.usable(LAKESHIRE, Team::Alliance),
        "ConditionID 20068"
    );
    assert!(
        network
            .route(
                STORMWIND,
                LAKESHIRE,
                &known(&[STORMWIND, LAKESHIRE]),
                Team::Alliance
            )
            .is_none()
    );
}

#[test]
fn horde_players_do_not_see_alliance_flight_points() {
    let Some(network) = network() else { return };
    assert!(network.visible(STORMWIND, Team::Alliance));
    assert!(!network.visible(STORMWIND, Team::Horde));
    let horde = network.nearest_node(0, DUNGAR, Team::Horde).unwrap();
    assert!(network.visible(horde, Team::Horde), "{horde}");
    assert_ne!(horde, STORMWIND);
}

#[test]
fn the_alliance_rides_a_gryphon() {
    let Some(network) = network() else { return };
    assert_eq!(network.mount_display(STORMWIND, Team::Alliance), Some(6852));
}
