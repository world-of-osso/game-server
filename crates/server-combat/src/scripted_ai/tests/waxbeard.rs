//! TDB1210 vehicle_template_accessory: Ol' Waxbeard 210149 rides his mount 210153
//! (seat0). VEHICLE_DEATH_CREDITS_BOUND_ACCESSORY: the mount's death completes 2829.
use super::*;

const MAP: u32 = 2651;
const MOUNT: u32 = 210153;
const RIDER: u32 = 210149;
const BIT: u32 = 3;

fn actors(h: &mut Harness, map: InMap, entry: u32) -> Vec<Entity> {
    h.app
        .world_mut()
        .query::<(Entity, &InMap, &shared::components::Npc)>()
        .iter(h.app.world())
        .filter(|(_, copy, npc)| **copy == map && npc.template_id == entry)
        .map(|(actor, _, _)| actor)
        .collect()
}

const WAXBEARD_GATE: u32 = 420154;
const CATACOMBS_GATE: u32 = 430177;
const BLAZIKON_DOOR: u32 = 430175;

/// Replicated collision of the copy's one `entry` door.
fn door_blocks(h: &mut Harness, map: InMap, entry: u32) -> bool {
    let doors: Vec<bool> = h
        .app
        .world_mut()
        .query::<(
            &InMap,
            &shared::protocol::GameObjectInfo,
            &shared::protocol::GameObjectCollision,
        )>()
        .iter(h.app.world())
        .filter(|(copy, info, _)| **copy == map && info.entry == entry)
        .map(|(_, _, collision)| collision.0)
        .collect();
    let [blocks] = doors[..] else {
        panic!("one {entry} door in the copy: {doors:?}");
    };
    blocks
}

fn position(h: &Harness, actor: Entity) -> Vec3 {
    let position = h.app.world().get::<Position>(actor).unwrap();
    Vec3::new(position.x, position.y, position.z)
}

#[test]
fn waxbeard_rides_the_filled_mount_and_its_death_completes_the_encounter() {
    let (mut h, _, map) =
        encounter_fixture(RIDER, MAP, BIT, 205, None, EncounterState::NotStarted, true);
    h.app
        .add_systems(Update, crate::vehicle::accessory::install_accessories);
    h.app
        .add_systems(FixedPostUpdate, crate::vehicle::accessory::seat_accessories);
    h.advance(0.1);
    let [mount] = actors(&mut h, map, MOUNT)[..] else {
        panic!("one filled mount");
    };
    let [rider] = actors(&mut h, map, RIDER)[..] else {
        panic!("one seated rider and no standing ground copy");
    };
    let seat = position(&h, rider) - position(&h, mount);
    assert!(seat.length() < 6.0, "rider sits on the mount: {seat:?}");
    h.app.world_mut().get_mut::<Position>(mount).unwrap().x += 10.0;
    h.advance(0.1);
    let moved = position(&h, rider) - position(&h, mount);
    assert!(
        moved.abs_diff_eq(seat, 0.01),
        "rider keeps its seat: {moved:?}"
    );
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), player, mount);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 2829 && end.success)
    );
    assert!(
        h.app.world().get_entity(rider).is_err(),
        "rider leaves with its mount"
    );
}

#[test]
fn waxbeard_gate_opens_when_the_encounter_is_done() {
    let (mut h, _, map) =
        encounter_fixture(RIDER, MAP, BIT, 205, None, EncounterState::NotStarted, true);
    h.app
        .add_systems(Update, crate::vehicle::accessory::install_accessories);
    h.app
        .add_systems(FixedPostUpdate, crate::vehicle::accessory::seat_accessories);
    h.advance(0.1);
    assert!(
        door_blocks(&mut h, map, WAXBEARD_GATE),
        "closed before the kill"
    );
    let [mount] = actors(&mut h, map, MOUNT)[..] else {
        panic!("one filled mount");
    };
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), player, mount);
    h.advance(0.1);
    assert!(
        !door_blocks(&mut h, map, WAXBEARD_GATE),
        "open after 2829 DONE"
    );
    assert!(
        door_blocks(&mut h, map, CATACOMBS_GATE),
        "the Candle King's gate stays closed"
    );
}

#[test]
fn blazikon_door_opens_when_its_encounter_is_done() {
    let (mut h, _, map) =
        encounter_fixture(RIDER, MAP, BIT, 205, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    assert!(
        door_blocks(&mut h, map, BLAZIKON_DOOR),
        "closed before Blazikon"
    );
    let world = h.app.world_mut();
    let blazikon = instance_script::boss_id_for_encounter(world, map, 2826).unwrap();
    assert!(instance_script::set_boss_state(
        world,
        map,
        blazikon,
        EncounterState::Done
    ));
    assert!(
        !door_blocks(&mut h, map, BLAZIKON_DOOR),
        "open after 2826 DONE"
    );
    assert!(door_blocks(&mut h, map, CATACOMBS_GATE));
}

#[test]
fn waxbeard_gate_fills_open_in_a_copy_saved_past_waxbeard() {
    let (mut h, _, map) = encounter_fixture(RIDER, MAP, BIT, 205, None, EncounterState::Done, true);
    h.advance(0.1);
    assert!(!door_blocks(&mut h, map, WAXBEARD_GATE));
    assert!(door_blocks(&mut h, map, CATACOMBS_GATE));
}
