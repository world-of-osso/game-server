//! A leader must not wait for an already-engaged tank to abandon its target.
use super::*;

#[test]
fn separated_leader_pursues_boss_while_tank_is_fighting() {
    let mut run = spawn_isolated_moloch(1);
    let boss = run.actors[&20375766];
    let tank = run.party.tank;
    let owner = run.party.owner;
    let distant = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == 20375706)
        .unwrap()
        .point();
    run.app.world_mut().entity_mut(owner).insert(Position {
        x: distant.x,
        y: distant.y,
        z: distant.z,
    });
    {
        let mut health = run.app.world_mut().get_mut::<Health>(boss).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
    }
    crate::combat::deal_direct_damage(run.app.world_mut(), tank, boss, 100.0);
    crate::combat::deal_direct_damage(run.app.world_mut(), owner, boss, 100.0);
    run.app.world_mut().entity_mut(owner).insert((
        crate::creature_ai::CreatureState::Pursuing {
            target: boss.to_bits(),
        },
        crate::networking::CombatTarget(Some(boss)),
    ));
    run.app.update();
    assert!(crate::followers::simulator::has_uncleared_fight(
        run.app.world(),
        owner
    ));
    run.next_cohesion_check = 0.0;
    assert!(
        run.wait_for_tank().unwrap(),
        "the same separation triggers the pre-pull hold"
    );
    let start = *run.app.world().get::<Position>(owner).unwrap();
    let tank_start = *run.app.world().get::<Position>(tank).unwrap();
    let tank_point = Vec3::new(tank_start.x, tank_start.y, tank_start.z);
    let initial_distance = distant.distance(tank_point);
    for _ in 0..300 {
        run.control().unwrap();
        run.app.update();
        let current = *run.app.world().get::<Position>(owner).unwrap();
        if Vec3::new(current.x, current.y, current.z).distance(tank_point) < initial_distance - 3.0
        {
            return;
        }
    }
    panic!(
        "leader remained at {start:?} while its tank fought the boss: {:?}",
        super::super::diagnostics::inspect_unit(run.app.world(), None, owner)
    );
}
