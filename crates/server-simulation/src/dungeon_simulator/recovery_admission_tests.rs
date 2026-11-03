//! Azure recovery must make an actual next-pull admission possible.
use super::*;

#[test]
#[ignore = "requires Azure Mythic graph, native navigation and frozen world database"]
fn holy_recovery_unblocks_next_pull_with_stalker_combat_reference() {
    let mut run = spawn_run_mode(
        70448,
        &mut None,
        Some(super::super::keystone::Options {
            map_id: 2515,
            level: 2,
            specs: [581, 257, 63, 262, 70],
        }),
    )
    .unwrap();
    let actors: Vec<_> = run.actors.values().copied().collect();
    for actor in actors {
        run.app
            .world_mut()
            .entity_mut(actor)
            .remove::<crate::creature_aggro::AggroRange>();
    }
    for _ in 0..200 {
        run.app.update();
    }
    let owner = run.party.owner;
    let tank = run.party.tank;
    let healer = run.party.members[1];
    let stalker = run.actors[&29001806];
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, stalker);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), healer, stalker);
    assert!(crate::combat_refs::has_active_reference_to(
        run.app.world(),
        owner,
        stalker
    ));
    let health = run.app.world_mut().get_mut::<Health>(owner).unwrap();
    let max = health.max;
    run.app
        .world_mut()
        .get_mut::<Health>(owner)
        .unwrap()
        .current = max * 0.49;
    let first = run.graph.route[0].pack;
    let sources = run
        .graph
        .packs
        .iter()
        .find(|pack| pack.id == first)
        .unwrap()
        .spawns
        .iter()
        .map(|spawn| spawn.guid)
        .collect();
    let before = crate::followers::simulator::pull(run.app.world_mut(), tank, &sources).unwrap();
    assert_eq!(before, (false, "Wait(Recovery)".to_owned()));
    run.current_pack = Some(first);
    run.record_pull(first, before.1);
    let mut key = run.keystone.take().unwrap();
    key.update(run.app.world_mut()).unwrap();
    let mut output = Vec::new();
    key.report_progress(
        &mut super::super::keystone::Progress::default(),
        run.app.world(),
        run.current_pack,
        &run.party,
        &run.report.pulls,
        &mut output,
    )
    .unwrap();
    let line = String::from_utf8(output).unwrap();
    assert_eq!(line.lines().count(), 1, "{line}");
    assert!(line.contains("KEYSTONE_PROGRESS"));
    assert!(line.contains("decision=\"Wait(Recovery)\""), "{line}");
    assert!(line.contains("party="));
    assert!(line.contains("enemies_in_combat="));
    run.keystone = Some(key);
    for _ in 0..6000 {
        run.app.update();
        if run.app.world().get::<Health>(owner).unwrap().current >= max * 0.5 {
            let decision =
                crate::followers::simulator::pull(run.app.world_mut(), tank, &sources).unwrap();
            if decision.0 {
                assert!(crate::combat_refs::has_active_reference_to(
                    run.app.world(),
                    owner,
                    stalker
                ));
                return;
            }
        }
    }
    panic!("healing did not unblock the native pull admission");
}
