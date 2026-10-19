//! TC evade hooks + instance_ramparts30s group357 reset in a loaded copy.
//! Native5 is explicitly wipe HARNESS SETUP, not authored boss rotation or balance.
use super::herald_call::{damage_to_percent, first_west};
use super::herald_intro::{actors_in_copy, kill_actor, spawn_second_copy};
use super::herald_intro2::{WEST, prepared_ramparts, received_casts};
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;

const GROUP_GUIDS: [u32; 3] = [20_202_693, 20_202_705, 20_202_706];
const DESCEND: Position = Position {
    x: -1429.1720,
    y: 84.34760,
    z: -1749.1290,
};
const WIPE_PROBE: u32 = 5;

fn alive_group(h: &mut Harness, map: InMap) -> Vec<(u32, Entity, u32)> {
    let mut rows: Vec<_> = h
        .app
        .world_mut()
        .query::<(
            Entity,
            &InMap,
            &crate::zones::CreatureOrigin,
            &shared::components::Npc,
            &Health,
        )>()
        .iter(h.app.world())
        .filter(|(_, copy, origin, _, health)| {
            **copy == map && GROUP_GUIDS.contains(&origin.guid) && health.current > 0.0
        })
        .map(|(actor, _, origin, npc, _)| (origin.guid, actor, npc.template_id))
        .collect();
    rows.sort_by_key(|row| row.0);
    rows
}

fn prepare_reset(
    difficulty: u32,
    keep_one_sentry: bool,
) -> (Harness, InMap, Entity, Entity, Option<Entity>) {
    let (mut h, map, nazan, vazruden, kept) = if keep_one_sentry {
        let (mut h, map, herald, sentries) = prepared_ramparts(difficulty);
        // Ordinary incoming player damage starts source JustEngagedWith/INTRO1;
        // one real Sentry death leaves count1 for the reset-counter control.
        damage_to_percent(&mut h, herald, 99.0);
        for _ in 0..320 {
            h.advance(0.05);
            let morphed = h
                .app
                .world()
                .get::<shared::components::Npc>(herald)
                .is_some_and(|npc| npc.template_id == 17536);
            if morphed && h.app.world().get::<Position>(herald) == Some(&WEST) {
                break;
            }
        }
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(herald)
                .unwrap()
                .template_id,
            17536
        );
        assert_eq!(h.app.world().get::<Position>(herald), Some(&WEST));
        kill_actor(&mut h, sentries[0]);
        h.advance(0.05);
        assert_eq!(
            h.app
                .world()
                .resource::<instance_script::InstanceScripts>()
                .0[&map]
                .ramparts_sentry_deaths,
            1
        );
        let vazruden = actors_in_copy(&mut h, 17537, map);
        assert_eq!(vazruden.len(), 1);
        (h, map, herald, vazruden[0], Some(sentries[1]))
    } else {
        let (h, map, nazan, vazruden) = first_west(difficulty);
        (h, map, nazan, vazruden, None)
    };
    let floor = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(DESCEND.x, DESCEND.y, DESCEND.z));
    let crate::ground::GroundProbe::Supported(floor) = floor else {
        panic!("actual reset-arena floor")
    };
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: DESCEND.x + 3.0,
        y: floor,
        z: DESCEND.z,
    });
    damage_to_percent(&mut h, nazan, 29.0);
    for _ in 0..180 {
        h.advance(0.05);
        if h.app
            .world()
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(nazan)
            .is_none()
        {
            break;
        }
    }
    assert_eq!(received_casts(&h, nazan, 20538), 1);
    assert!(
        h.app
            .world()
            .get::<crate::scripted_ai::hellfire_ramparts::HeraldUnkillable>(nazan)
            .is_none()
    );
    assert_eq!(
        h.app.world().get::<ReactState>(nazan),
        Some(&ReactState::Aggressive)
    );
    h.app.init_resource::<ObservedCombatEvents>();
    (h, map, nazan, vazruden, kept)
}

fn wipe_harness_player(h: &mut Harness, nazan: Entity) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(WIPE_PROBE)
        .unwrap();
    assert_eq!(
        spell.effects[0].effect, 1,
        "native instakill HARNESS SETUP, no HP write"
    );
    let player = h.player();
    crate::spell_cast::creature_cast(h.app.world_mut(), nazan, WIPE_PROBE, Some(player), false)
        .expect("ordinary native wipe-probe admission");
    assert_eq!(h.health(player).current, 0.0);
    assert!(
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.kind == CombatLogKind::Death
                    && event.source == Some(nazan.to_bits())
                    && event.target == Some(player.to_bits())
            }),
        "actual native player death, never a fake evade Hook"
    );
}

#[test]
fn encounter_coverage_herald_reset_native_group_and_wipe_qualification() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let flags: u32 = conn
        .query_row(
            "SELECT groupFlags FROM tdb_spawn_group_template WHERE groupId=357",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        flags, 0,
        "regular initial population; not a manual-spawn conversion"
    );
    let mut statement=conn.prepare("SELECT spawnId FROM tdb_spawn_group WHERE groupId=357 AND spawnType=0 ORDER BY spawnId").unwrap();
    let ids: Vec<u32> = statement
        .query_map([], |row| row.get::<_, u32>(0))
        .unwrap()
        .map(|row| row.unwrap() + 20_000_000)
        .collect();
    assert_eq!(ids, GROUP_GUIDS);
    let (mut h, _, nazan, _, _) = prepare_reset(1, false);
    wipe_harness_player(&mut h, nazan);
}

#[test]
fn encounter_coverage_herald_reset_ordinary_damage_wipe_bounded_control() {
    let (mut h, _, nazan, vazruden, _) = prepare_reset(2, false);
    let player = h.player();
    let before = h.health(player).current;
    for _ in 0..3600 {
        h.advance(0.05);
        if h.health(player).current <= 0.0 {
            break;
        }
    }
    let after = h.health(player).current;
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event|event.kind==CombatLogKind::Damage && event.amount>0
        && matches!(event.source,Some(source) if source==nazan.to_bits() || source==vazruden.to_bits())));
    assert_eq!(received_casts(&h, nazan, WIPE_PROBE), 0);
    if after <= 0.0 {
        assert!(events.iter().any(|event|event.kind==CombatLogKind::Death && event.target==Some(player.to_bits())
            && matches!(event.source,Some(source) if source==nazan.to_bits() || source==vazruden.to_bits())));
        println!(
            "ordinary boss damage reached real wipe within180s; unchanged fixture initialHP{before}"
        );
    } else {
        println!(
            "HERALD_ORDINARY_DAMAGE_WIPE_NOT_REACHED_WITHIN_180S: unchanged encounter-buffer initialHP{before}, remainingHP{after}; native damage observed, no stat/HP overrides"
        );
    }
}

fn fresh_listener(h: &mut Harness, map: InMap, position: Position) {
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let (player, connection) = spawn_player(&mut h.app, map, [x, y, z, 0.0], 19001);
    let dead = h.players.remove(0);
    h.players.insert(0, (player, connection, Inbox::default()));
    h.players.push(dead);
}

fn assert_reset(difficulty: u32, keep_one_sentry: bool) {
    let (mut h, map, nazan, vazruden, kept) = prepare_reset(difficulty, keep_one_sentry);
    let other_map = spawn_second_copy(&mut h);
    let other_before = alive_group(&mut h, other_map);
    assert_eq!(other_before.len(), 3);
    let failed_before = h.inbox(0).ends.iter().filter(|end| !end.success).count();
    wipe_harness_player(&mut h, nazan);
    let mut failed_at = None;
    for _ in 0..60 {
        h.advance(0.05);
        if h.app
            .world()
            .resource::<instance_script::InstanceScripts>()
            .0[&map]
            .bosses[1]
            .state
            == EncounterState::Fail
        {
            failed_at.get_or_insert(h.app.world().resource::<Time<Fixed>>().elapsed());
            if h.app
                .world()
                .get::<crate::combat::DeathState>(nazan)
                .is_some()
                && h.app.world().get_entity(vazruden).is_err()
            {
                break;
            }
        }
    }
    let failed_at = failed_at
        .expect("real no-valid-target evade must reach source FAIL, not generic home return only");
    assert_eq!(
        h.inbox(0).ends.iter().filter(|end| !end.success).count(),
        failed_before + 1
    );
    assert!(h.inbox(0).ends.iter().all(|end| !end.success));
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(nazan)
            .is_some()
    );
    assert!(
        h.app.world().get_entity(vazruden).is_err(),
        "actual TempSummon evade unsummons it"
    );
    // TC InstanceMap2993: Map::Update(unit hooks) precedes copy Events.Update.
    // The scheduling tick consumes50ms of the real30_000ms source EventMap.
    while h.app.world().resource::<Time<Fixed>>().elapsed() - failed_at
        < Duration::from_millis(29900)
    {
        h.advance(0.05);
        let alive = alive_group(&mut h, map);
        assert_eq!(
            alive.len(),
            usize::from(kept.is_some()),
            "no early reset through source+29900ms"
        );
        assert_eq!(alive_group(&mut h, other_map), other_before);
    }
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - failed_at,
        Duration::from_millis(29900)
    );
    h.advance(0.05);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed() - failed_at,
        Duration::from_millis(29950)
    );
    let restored = alive_group(&mut h, map);
    assert_eq!(
        restored
            .iter()
            .map(|row| (row.0, row.2))
            .collect::<Vec<_>>(),
        vec![
            (20_202_693, 17307),
            (20_202_705, 17517),
            (20_202_706, 17517)
        ],
        "actual group357 ignoreRespawn reset"
    );
    if let Some(kept) = kept {
        assert!(
            restored.iter().any(|row| row.1 == kept),
            "ignoreRespawn is not force: keep living member identity"
        );
    }
    assert_eq!(alive_group(&mut h, other_map), other_before);
    assert!(actors_in_copy(&mut h, 17537, map).is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<instance_script::InstanceScripts>()
            .0[&map]
            .ramparts_sentry_deaths,
        0
    );
    h.advance(0.05); // Normal fresh-actor initialization; no fabricated callback.
    let herald = restored[0].1;
    let patrol_start = *h.app.world().get::<Position>(herald).unwrap();
    h.advance(3.0);
    assert_ne!(
        h.app.world().get::<Position>(herald),
        Some(&patrol_start),
        "fresh source patrol really advances"
    );
    let lethal = h.health(herald).max * 2.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), herald, herald, lethal);
    assert_eq!(
        h.health(herald).current,
        1.0,
        "fresh Herald restores actual lethal min1 interception"
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(herald)
            .is_none()
    );
    let sentries: Vec<_> = restored
        .iter()
        .filter(|row| row.2 == 17517)
        .map(|row| row.1)
        .collect();
    let near = *h.app.world().get::<Position>(sentries[0]).unwrap();
    // New native fixture player after the wipe; no resurrection/health write or
    // claim about the excluded player re-entry/persistence pipeline.
    fresh_listener(
        &mut h,
        map,
        Position {
            x: near.x + 500.0,
            ..near
        },
    );
    kill_actor(&mut h, sentries[0]);
    h.advance(0.05);
    assert_eq!(
        h.app.world().get::<ReactState>(herald),
        Some(&ReactState::Aggressive),
        "old count1 must not leak into new intro"
    );
    kill_actor(&mut h, sentries[1]);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(near);
    let before = *h.app.world().get::<Position>(herald).unwrap();
    h.advance(0.1);
    assert_eq!(
        h.app.world().get::<ReactState>(herald),
        Some(&ReactState::Passive)
    );
    let after = *h.app.world().get::<Position>(herald).unwrap();
    let center = Vec3::new(-1410.5204, 89.05998, -1740.5852);
    assert!(
        Vec3::new(after.x, after.y, after.z).distance(center)
            < Vec3::new(before.x, before.y, before.z).distance(center),
        "fresh source INTRO1 physically advances toward center"
    );
    assert_eq!(
        h.inbox(0)
            .chat
            .iter()
            .filter(|line| line.channel == ChatType::MonsterYell(herald.to_bits()))
            .count(),
        1,
        "one fresh source intro speech"
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(h.inbox(0).ends.iter().all(|end| !end.success));
}

#[test]
#[ignore = "HERALD_EVADE_RESET_UNIMPLEMENTED: real evade and30s RED first"]
fn encounter_coverage_herald_reset_normal() {
    assert_reset(1, false);
}
#[test]
#[ignore = "HERALD_EVADE_RESET_UNIMPLEMENTED: real evade and30s RED first"]
fn encounter_coverage_herald_reset_heroic() {
    assert_reset(2, false);
}
#[test]
#[ignore = "HERALD_EVADE_RESET_UNIMPLEMENTED: ignoreRespawn alive/counter RED first"]
fn encounter_coverage_herald_reset_keep_alive_normal() {
    assert_reset(1, true);
}
#[test]
#[ignore = "HERALD_EVADE_RESET_UNIMPLEMENTED: ignoreRespawn alive/counter RED first"]
fn encounter_coverage_herald_reset_keep_alive_heroic() {
    assert_reset(2, true);
}
