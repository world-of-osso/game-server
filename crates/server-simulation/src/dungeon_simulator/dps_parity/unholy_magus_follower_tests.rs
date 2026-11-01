//! Production level-70 Unholy follower Magus and Lord actions without the MID2 set.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;
use std::collections::{HashMap, HashSet};

const MAGUS: u32 = 163366;
const LORD: u32 = 264321;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_unholy_magus_and_lord_cast_frostbolt_without_mid2_set() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 252, 70);
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    for _ in 0..5 {
        app.update();
    }
    assert!(!crate::auras::has_aura(app.world(), actor, 1296654));
    start_fixed_time_fight(
        &mut app,
        owner,
        actor,
        target,
        f64::from(FIXED_FIGHT_FRAMES) / 20.0,
    );
    let mut entries: HashMap<u64, u32> = HashMap::new();
    let mut hits: Vec<(u32, u64, u32, f64)> = Vec::new();
    let mut seen = 0;
    for _ in 0..FIXED_FIGHT_FRAMES {
        app.update();
        let world = app.world_mut();
        for (unit, npc) in world
            .query::<(Entity, &shared::components::Npc)>()
            .iter(world)
        {
            if matches!(npc.template_id, MAGUS | LORD) {
                entries.insert(unit.to_bits(), npc.template_id);
            }
        }
        let now = crate::spell_cast::now(world);
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[seen..] {
            let Some(&entry) = event.source.and_then(|source| entries.get(&source)) else {
                continue;
            };
            if event.kind == CombatLogKind::Damage && event.amount > 0 {
                hits.push((
                    entry,
                    event.source.unwrap(),
                    event.spell_id.unwrap_or(0),
                    now,
                ));
            }
        }
        seen = events.len();
    }
    let spells: HashSet<_> = hits
        .iter()
        .map(|&(entry, _, spell, _)| (entry, spell))
        .collect();
    // SimC76f8118 sc_death_knight.cpp4598–4607,4664–4670: pre-MID2 pet actions.
    assert!(spells.contains(&(MAGUS, 317792)), "{spells:?}");
    assert!(spells.contains(&(MAGUS, 317791)), "{spells:?}");
    assert!(spells.contains(&(LORD, 1292107)), "{spells:?}");
    assert!(
        spells
            .iter()
            .all(|&(_, spell)| !matches!(spell, 1297055 | 1297086 | 1297091)),
        "{spells:?}"
    );
    // SimC76f8118 cpp4429–4433: each Magus recasts Frostbolt after its 6 s slow.
    let mut last: HashMap<u64, f64> = HashMap::new();
    for &(_, source, _, at) in hits.iter().filter(|hit| hit.0 == MAGUS && hit.2 == 317792) {
        if let Some(previous) = last.insert(source, at) {
            assert!(at - previous >= 5.95, "Magus Frostbolt {previous} -> {at}");
        }
    }
}
