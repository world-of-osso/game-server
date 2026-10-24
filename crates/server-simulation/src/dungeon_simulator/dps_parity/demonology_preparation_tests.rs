//! Real native out-of-combat preparation and first-pull lifecycle; no literal controller.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native level90 actual Hand of Guldan imp attack boundary; private guardian-template DB required"]
fn dps_parity_demo_native_guldan_imps_attack_the_selected_dummy() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare(266, 3).unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    engage(&mut run);
    let mut observed = std::collections::HashSet::new();
    for tick in 0..1200 {
        run.app.update();
        let world = run.app.world_mut();
        let imps: Vec<_> = world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .filter(|(_, summon)| summon.owner == run.actor && summon.spell_id == 104317)
            .map(|(entity, _)| entity)
            .collect();
        for imp in imps {
            if observed.insert(imp) {
                let spell = world
                    .resource::<crate::spell_info::SpellStore>()
                    .get(104318)
                    .unwrap()
                    .clone();
                let owner_target = world
                    .get::<crate::networking::CombatTarget>(run.actor)
                    .and_then(|target| target.0);
                let admission =
                    crate::spell_cast::read_native_cast_start(world, imp, &spell, run.targets[0]);
                println!(
                    "DEMO_IMP tick={tick} entity={imp:?} owner_target={owner_target:?} admission={admission:?}"
                );
                println!(
                    "DEMO_IMP_STATE level={:?} owner_level={:?} faction={:?} map={:?} target_map={:?} enemy={}",
                    world.get::<shared::components::UnitLevel>(imp),
                    world.get::<shared::components::Level>(run.actor),
                    world.get::<shared::components::UnitFactionTemplate>(imp),
                    world.get::<crate::maps::InMap>(imp),
                    world.get::<crate::maps::InMap>(run.targets[0]),
                    crate::spell_cast::targets::relation_matches(
                        world,
                        imp,
                        run.targets[0],
                        crate::spell_cast::targets::Relation::Enemy
                    )
                );
            }
        }
    }
    assert!(
        !observed.is_empty(),
        "ordinary native Hand of Guldan must actually spawn Wild Imps"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(104318)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hits > 0,
        "actual level90 native Wild Imps must attack the selected dummy; observed {} actors",
        observed.len()
    );
}

#[test]
#[ignore = "native first10s ST/3T engagement regression; private spec266 inputs required"]
fn dps_parity_demo_native_first_pull_survives_inflight_precombat_cast() {
    let _seed = crate::random::seed_scope(70448);
    let mut outcomes = Vec::new();
    for count in [1, 3] {
        let mut run = prepare(266, count).unwrap();
        engage(&mut run);
        let started = crate::spell_cast::now(run.app.world());
        let mut events_seen = 0;
        let mut casts = Vec::new();
        let mut samples = Vec::new();
        for tick in 1..=200 {
            run.app.update();
            let world = run.app.world();
            let now = crate::spell_cast::now(world) - started;
            let events = &world.resource::<ObservedCombatEvents>().0;
            casts.extend(events[events_seen..].iter().filter_map(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::CastStart | CombatLogKind::CastSuccess
                )
                .then(|| json!({"at": now, "event": event}))
            }));
            events_seen = events.len();
            if tick % 20 == 0 {
                let owner = fixture_player(&run);
                samples.push(json!({"at": now,
                    "cast": format!("{:?}", world.get::<shared::casting::CastState>(run.actor)),
                    "target": world.get::<crate::networking::CombatTarget>(run.actor).and_then(|target| target.0).map(Entity::to_bits),
                    "owner_reference": crate::combat_refs::has_active_reference_to(world, owner, run.targets[0]),
                    "actor_reference": crate::combat_refs::has_active_reference_to(world, run.actor, run.targets[0]),
                    "pet": crate::hunter_pet::summoned_pet(world, run.actor).map(Entity::to_bits)}));
            }
        }
        let damage = report::combat(run.app.world(), &run.damage_targets, 10.0)["total_damage"]
            .as_u64()
            .unwrap();
        outcomes
            .push(json!({"targets": count, "damage": damage, "casts": casts, "samples": samples}));
    }
    let path = inputs::directory()
        .unwrap()
        .join("266.first-engagement-10s.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&outcomes).unwrap()).unwrap();
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome["damage"].as_u64().unwrap() > 0),
        "native first pull must reach actual damage in ST and3T; trace {}",
        path.display()
    );
}

fn load_authored_guardian_arena(run: &mut Run) -> shared::components::Position {
    // This authored2669 scene has no frozen Stockade34 route graph.
    run.app
        .world_mut()
        .remove_resource::<crate::dungeon_simulator::Space>();
    crate::test_data::install_authored_guardian_arena(&mut run.app)
}

fn place_guardian_control_in_arena(run: &mut Run, origin: shared::components::Position) {
    use shared::components::{Position, Rotation};
    let map = crate::maps::InMap {
        map_id: 2669,
        instance_id: 1,
    };
    let controller = fixture_player(run);
    crate::followers::simulator::relocate_parity_party(run.app.world_mut(), controller, map)
        .unwrap();
    run.app.world_mut().entity_mut(controller).insert(Position {
        x: origin.x + 20.0,
        ..origin
    });
    run.app.world_mut().entity_mut(run.actor).insert((
        Position {
            x: origin.x - 5.0,
            ..origin
        },
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    if let Some(pet) = crate::hunter_pet::summoned_pet(run.app.world(), run.actor) {
        run.app.world_mut().entity_mut(pet).insert(Position {
            x: origin.x - 3.0,
            ..origin
        });
    }
    for (index, target) in run.targets.iter().enumerate() {
        run.app.world_mut().entity_mut(*target).insert((
            Position {
                z: origin.z - index as f32,
                ..origin
            },
            map,
        ));
    }
    run.app.update();
}

fn prepare_authored_guardian_control() -> Run {
    let mut run = prepare_manual_guardian_control();
    let origin = load_authored_guardian_arena(&mut run);
    place_guardian_control_in_arena(&mut run, origin);
    assert!(
        crate::spell_cast::targets::is_alive(run.app.world(), run.actor),
        "authored scene relocation must retain the living admitted caster"
    );
    run
}

fn cast_native_antoran_tyrant(run: &mut Run) -> Entity {
    let actor = run.actor;
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1250921),
        "selected Antoran Armaments must be active before the native summon"
    );
    cast_and_finish(run, actor, 265187);
    let world = run.app.world_mut();
    let tyrants: Vec<_> = world
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == actor && summon.spell_id == 265187 && npc.template_id == 250289
        })
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(
        tyrants.len(),
        1,
        "ordinary native265187 must create one actual250289"
    );
    tyrants[0]
}

#[test]
#[ignore = "actual279910/143622 cast-list and energy-fade boundary; private native143622 inputs required"]
fn dps_parity_demo_native_alternate_imp_casts_and_exhausts_before_native_ttl() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 279910);
    let world = run.app.world_mut();
    let imps: Vec<_> = world
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == actor && summon.spell_id == 279910 && npc.template_id == 143622
        })
        .map(|(entity, summon, _)| (entity, summon.expires_at))
        .collect();
    assert_eq!(
        imps.len(),
        1,
        "actual279910 must spawn its own143622, not a55659 clone"
    );
    let (imp, expires_at) = imps[0];
    let display = world
        .get::<shared::components::ModelDisplay>(imp)
        .unwrap()
        .display_id;
    assert!(
        [77405, 77406, 77407].contains(&display),
        "native143622 model choice"
    );
    let initial_energy =
        crate::spell_cast::powers::power_value(world, imp, shared::components::PowerType::Energy)
            .map(|pool| pool.0);
    let cost = world
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap()
        .power_costs
        .iter()
        .find(|cost| cost.power_type == shared::components::PowerType::Energy as i32)
        .unwrap()
        .mana_cost;
    let started = crate::spell_cast::now(world);
    let events_before = world.resource::<ObservedCombatEvents>().0.len();
    for _ in 0..240 {
        run.app.update();
    }
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0[events_before..];
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    let damage: i64 = events
        .iter()
        .filter(|event| {
            event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum();
    let remaining = world
        .get::<crate::spell_cast::summon::SummonedCreature>(imp)
        .is_some();
    let path = inputs::directory()
        .unwrap()
        .join("266.alternate-imp-lifecycle.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "root": 279910, "entry": 143622, "display": display, "native_expires_at": expires_at,
            "started_at": started, "observed_at": crate::spell_cast::now(world),
            "initial_energy": initial_energy, "native_cost": cost, "casts": casts,
            "damage": damage, "remaining": remaining
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        initial_energy,
        Some(100.0),
        "native143622 needs its source-bound caster pool; {}",
        path.display()
    );
    assert_eq!(cost, 20, "334591 cost policy is excluded");
    assert_eq!(
        casts,
        5,
        "real native100Energy/20cost must exhaust without regeneration; {}",
        path.display()
    );
    assert!(
        damage > 0 && !remaining,
        "real alternate Imp must deal damage and energy-fade before20s; {}",
        path.display()
    );
}

fn prepare_alternate_identity_control() -> Run {
    let mut run = prepare_authored_guardian_control();
    // Explicit approved source-binding fixture isolates identity from the
    // separately tested production list policy. This is not an authored TDB row.
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::guardian::GuardianSpells>()
        .spells
        .insert(143622, vec![104318]);
    run
}

fn trigger_native_imp(run: &mut Run, root: u32) -> Entity {
    let previous: std::collections::HashSet<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .filter(|(_, summon)| summon.owner == run.actor)
        .map(|(entity, _)| entity)
        .collect();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, root, Some(run.actor), 0);
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(entity, summon)| {
            summon.owner == run.actor && summon.spell_id == root && !previous.contains(entity)
        })
        .map(|(entity, _)| entity)
        .expect("native triggered root must produce its own new actor")
}

#[test]
#[ignore = "bound actual143622 must energy-fade before native20s; private native143622 inputs required"]
fn dps_parity_demo_alternate_imp_identity_fades_with_explicit_cast_binding() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_alternate_identity_control();
    let imp = trigger_native_imp(&mut run, 279910);
    let initial = crate::spell_cast::powers::power_value(
        run.app.world(),
        imp,
        shared::components::PowerType::Energy,
    )
    .unwrap()
    .0;
    for _ in 0..240 {
        run.app.update();
    }
    let world = run.app.world();
    let casts = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    let remaining = world
        .get::<crate::spell_cast::summon::SummonedCreature>(imp)
        .is_some();
    let path = inputs::directory()
        .unwrap()
        .join("266.alternate-identity-fade.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&json!({"initial_energy": initial,
        "casts": casts, "remaining_before20s": remaining, "fixture_binding": "approved143622→104318"})).unwrap()).unwrap();
    assert_eq!(
        casts,
        5,
        "existing native100/20 resource path; {}",
        path.display()
    );
    assert!(
        !remaining,
        "alternate identity must energy-fade, not idle until20s; {}",
        path.display()
    );
}

#[test]
#[ignore = "actual alternate WildImps must join native Core and Tyrant roster; private143622 inputs required"]
fn dps_parity_demo_alternate_imp_identity_earns_core_and_updates_tyrant_roster() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_alternate_identity_control();
    // Existing sourced Imp fade PRD guarantees a proc by21 natural fades.
    for _ in 0..21 {
        trigger_native_imp(&mut run, 279910);
    }
    let tyrant = cast_native_antoran_tyrant(&mut run);
    let stacks = |world: &World, unit: Entity, spell| {
        world
            .get::<crate::auras::ActiveAuras>(unit)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
            .map_or(0, |aura| aura.stacks)
    };
    let before = stacks(run.app.world(), tyrant, 1276788);
    for _ in 0..240 {
        run.app.update();
    }
    let after = stacks(run.app.world(), tyrant, 1276788);
    let core = stacks(run.app.world(), run.actor, 264173);
    let path = inputs::directory()
        .unwrap()
        .join("266.alternate-identity-core-roster.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({"native_alternate_roots": 21,
        "tyrant_stacks_before": before, "tyrant_stacks_after": after, "natural_core_stacks": core}))
        .unwrap(),
    )
    .unwrap();
    assert!(
        before > after,
        "real alternate fades must reduce actual Tyrant roster; {}",
        path.display()
    );
    assert!(
        core > 0,
        "21native alternate fades must earn Core through existing PRD; {}",
        path.display()
    );
}

#[test]
#[ignore = "native alternate and HoG Imps share qualified SP identity; private143622 inputs required"]
fn dps_parity_demo_alternate_imp_identity_matches_native_hog_imp_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_alternate_identity_control();
    let primary = trigger_native_imp(&mut run, 104317);
    let alternate = trigger_native_imp(&mut run, 279910);
    let variance = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .variance;
    for _ in 0..240 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let first_hit = |source: Entity| {
        events
            .iter()
            .find(|event| {
                event.source == Some(source.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(104318)
                    && event.kind == CombatLogKind::Damage
                    && !event.crit
            })
            .expect("both real Imps must produce noncritical104318 damage")
            .amount as f32
    };
    let primary_hit = first_hit(primary);
    let alternate_hit = first_hit(alternate);
    let ratio_bound = (1.0 + variance) / (1.0 - variance);
    let path = inputs::directory()
        .unwrap()
        .join("266.alternate-identity-power.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({"primary_hit": primary_hit,
        "alternate_hit": alternate_hit, "native_variance": variance, "ratio_bound": ratio_bound}))
        .unwrap(),
    )
    .unwrap();
    assert!(
        alternate_hit >= primary_hit / ratio_bound - 2.0
            && alternate_hit <= primary_hit * ratio_bound + 2.0,
        "native same-owner/same-spell actors must share qualified WildImp power within native variance/integer rounding; {}",
        path.display()
    );
}

#[test]
#[ignore = "future native alternate cast list must win at runtime; private143622 inputs required"]
fn dps_parity_demo_alternate_imp_native_cast_list_does_not_inject_policy_energy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    // Hypothetical future native list uses actual Mana-cost ShadowBolt686.
    // This is not a published143622 row; never inject104318/Energy.
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::guardian::GuardianSpells>()
        .spells
        .insert(143622, vec![686]);
    let imp = trigger_native_imp(&mut run, 279910);
    assert!(
        crate::spell_cast::powers::power_value(
            run.app.world(),
            imp,
            shared::components::PowerType::Energy
        )
        .is_none()
    );
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(imp)
            .is_some()
    );
    assert!(
        !run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(imp.to_bits())
                    && event.spell_id == Some(104318)
                    && event.kind == CombatLogKind::CastSuccess
            }),
        "native list must not acquire a project attack or its Energy pool"
    );
}

fn restart_selected_inner_demons(run: &mut Run) -> f64 {
    let actor = run.actor;
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 267216,
    );
    // Let the real KnownSpell passive reapply. A manual Cast-source aura would
    // coexist with it, manufacturing two schedulers rather than testing one.
    run.app.update();
    let count = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == 267216)
        .count();
    assert_eq!(
        count, 1,
        "one actual selected passive aura, not two fixture sources"
    );
    crate::spell_cast::now(run.app.world())
}

#[test]
#[ignore = "native267216 first+12s periodic association; private native143622 inputs required"]
fn dps_parity_demo_inner_demons_uses_native_period_without_tick_zero() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    let started = restart_selected_inner_demons(&mut run);
    let period_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(267216)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert_eq!(period_ms, 12000, "authored native226 period");
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let mut births = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..500 {
        run.app.update();
        let world = run.app.world_mut();
        let now = crate::spell_cast::now(world) - started;
        for (entity, summon, npc) in world
            .query::<(
                Entity,
                &crate::spell_cast::summon::SummonedCreature,
                &shared::components::Npc,
            )>()
            .iter(world)
        {
            if summon.owner == actor && summon.spell_id == 279910 && seen.insert(entity) {
                births.push(json!({"at": now, "entry": npc.template_id, "root": summon.spell_id}));
            }
        }
    }
    let path = inputs::directory()
        .unwrap()
        .join("266.inner-demons-periodic.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({"period_ms": period_ms, "births": births,
        "events": &run.app.world().resource::<ObservedCombatEvents>().0[before..]}))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        births.len(),
        2,
        "two native ticks in25s; no tick_zero/random phase; {}",
        path.display()
    );
    for (birth, expected) in births.iter().zip([12.0, 24.0]) {
        assert!(
            (birth["at"].as_f64().unwrap() - expected).abs() <= 0.051,
            "native tick phase: {birth}"
        );
        assert_eq!(birth["entry"], 143622);
    }
}

#[test]
#[ignore = "unselected native267216 must not produce alternate Imps; private143622 inputs required"]
fn dps_parity_demo_inner_demons_unselected_does_not_summon() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    inputs::install_traits(run.app.world_mut(), run.actor, &[]).unwrap();
    for _ in 0..4 {
        run.app.update();
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 267216));
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..500 {
        run.app.update();
    }
    assert!(
        !run.app.world().resource::<ObservedCombatEvents>().0[before..]
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(279910)
                    && event.kind == CombatLogKind::Summon
            }),
        "unselected owner must not gain project periodic summons"
    );
}

fn cast_native_inner_demons_script_control(
    world: &mut World,
    unit: Entity,
    _aura: &crate::auras::AuraInstance,
    index: u8,
) {
    if index == 0 {
        crate::spell_cast::cast_triggered(world, unit, 104317, Some(unit), 1);
    }
}

#[test]
#[ignore = "native trigger/script overrides project periodic association; private143622 inputs required"]
fn dps_parity_demo_inner_demons_native_trigger_or_script_wins() {
    let _seed = crate::random::seed_scope(70448);
    for scripted in [false, true] {
        let mut run = prepare_authored_guardian_control();
        let actor = run.actor;
        if scripted {
            // Hypothetical native registration fixture, not an observed TC hook.
            run.app
                .world_mut()
                .resource_mut::<crate::auras::scripts::AuraScripts>()
                .0
                .insert(
                    267216,
                    crate::auras::scripts::AuraScript {
                        on_periodic: Some(cast_native_inner_demons_script_control),
                        ..Default::default()
                    },
                );
        } else {
            // Hypothetical future native trigger, retaining the actual aura period.
            let mut store = run
                .app
                .world_mut()
                .resource_mut::<crate::spell_info::SpellStore>();
            let mut spell = store.get(267216).unwrap().clone();
            spell
                .effects
                .iter_mut()
                .find(|effect| effect.index == 0)
                .unwrap()
                .trigger_spell = 104317;
            store.replace_fixture_spell(spell);
        }
        restart_selected_inner_demons(&mut run);
        let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
        for _ in 0..500 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[before..];
        let summons = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Summon
                })
                .count()
        };
        assert_eq!(
            summons(104317),
            2,
            "native authority runs exactly once per tick; scripted={scripted}"
        );
        assert_eq!(
            summons(279910),
            0,
            "project association must not add a second summon; scripted={scripted}"
        );
    }
}

#[test]
#[ignore = "native265187 Antoran autoattack replacement on authored ground/LOS; private spec266 inputs required"]
fn dps_parity_demo_native_antoran_tyrant_uses_burning_cleave_autoattacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let tyrant = cast_native_antoran_tyrant(&mut run);
    let world = run.app.world_mut();
    let armaments = crate::auras::has_aura(world, tyrant, 1251000);
    let cleave = world
        .resource::<crate::spell_info::SpellStore>()
        .get(1264093)
        .unwrap()
        .clone();
    let admission = format!(
        "{:?}",
        crate::spell_cast::read_native_cast_start(world, tyrant, &cleave, run.targets[0])
    );
    let events_before = world.resource::<ObservedCombatEvents>().0.len();
    for _ in 0..120 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[events_before..];
    // Full-mask triggered swings publish damage, not CastSuccess. One selected
    // victim proves repeated launches rather than several targets of one AoE.
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(tyrant.to_bits())
                && event.spell_id == Some(1264093)
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .count();
    let damage: i64 = events
        .iter()
        .filter(|event| {
            event.source == Some(tyrant.to_bits())
                && event.spell_id == Some(1264093)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum();
    let path = inputs::directory()
        .unwrap()
        .join("266.antoran-autoattack-boundary.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "native_root": 265187, "entry": 250289, "actor": tyrant.to_bits(),
            "armaments_aura_at_spawn": armaments, "initial_cast_admission": admission,
            "window_seconds": 6, "selected_target_cleave_hits": hits, "cleave_damage": damage
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        armaments,
        "native Antoran must carry its1251000 passive; boundary {}",
        path.display()
    );
    assert!(
        hits > 1 && damage > 0,
        "native autoattack replacement must execute repeatedly with real damage, not once through an untimed spell list; boundary {}",
        path.display()
    );
}

#[test]
#[ignore = "native Aura361 on an actual melee guardian must replace white swings; private spec266 inputs required"]
fn dps_parity_demo_native_armaments_aura_replaces_guardian_white_swings() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 104316);
    let world = run.app.world_mut();
    let guardian = world
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == actor && summon.spell_id == 193331)
        .map(|(entity, _)| entity)
        .expect("ordinary104316 must produce its native melee guardian");
    // Isolate the native melee-consumer boundary independently of the missing
    // summon-passive grant. Apply the actual DB2 aura, not a synthetic modifier.
    crate::spell_cast::cast_triggered(run.app.world_mut(), guardian, 1251000, Some(guardian), 0);
    assert!(crate::auras::has_aura(run.app.world(), guardian, 1251000));
    let events_before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..120 {
        run.app.update();
    }
    let events: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[events_before..]
        .iter()
        .filter(|event| event.source == Some(guardian.to_bits()))
        .collect();
    let hits = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(1264093)
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
        })
        .count();
    let white_hits = events
        .iter()
        .filter(|event| event.spell_id.is_none() && event.kind == CombatLogKind::Damage)
        .count();
    let path = inputs::directory()
        .unwrap()
        .join("266.armaments-melee-consumer.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "native_aura": 1251000, "aura_type": 361, "entry": 98035,
            "window_seconds": 6, "selected_target_cleave_hits": hits, "white_hits": white_hits, "events": events
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        hits > 1,
        "native361 must replace each swing, not one untimed list cast; boundary {}",
        path.display()
    );
    assert_eq!(
        white_hits,
        0,
        "TC override replaces white damage; boundary {}",
        path.display()
    );
}

#[test]
#[ignore = "full native104316 dispatch on authored ground/LOS; private spec266 inputs required"]
fn dps_parity_demo_native_call_dreadstalkers_dispatches_both_authored_children() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 104316);
    for _ in 0..40 {
        run.app.update();
    }
    let world = run.app.world_mut();
    let mut children: Vec<_> = world.query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &shared::components::Npc, &shared::components::UnitLevel)>()
        .iter(world)
        .filter(|(_, summon, _, _)| summon.owner == actor && matches!(summon.spell_id, 193331 | 193332))
        .map(|(entity, summon, npc, level)| json!({"entity": entity.to_bits(), "root": summon.spell_id, "entry": npc.template_id, "level": level.0}))
        .collect();
    children.sort_by_key(|child| child["root"].as_u64().unwrap());
    let path = inputs::directory()
        .unwrap()
        .join("266.dreadstalker-root-dispatch.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(
            &json!({"map": 2669, "authored_spawn": 216619, "children": &children}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        children.len(),
        2,
        "native104316 must dispatch both native Effect28 children; trace {}",
        path.display()
    );
    assert_eq!(
        children
            .iter()
            .map(|child| child["root"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [193331, 193332]
    );
    let owner_level = world.get::<shared::components::Level>(actor).unwrap().0;
    assert!(
        children
            .iter()
            .all(|child| child["entry"] == 98035 && child["level"] == owner_level)
    );
}

#[test]
#[ignore = "selected Demoniac owner earns Core from native Dread retirement; private spec266 inputs required"]
fn dps_parity_demo_native_dreadstalker_retirement_earns_both_core_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    let core_stacks = |world: &World| {
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 264173))
            .map_or(0, |aura| aura.stacks)
    };
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap()
            .contains(426115),
        "actual selected Demoniac owner, not a globally enabled guardian proc"
    );
    let chance = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(267102)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .base_points;
    assert_eq!(
        chance, 100.0,
        "native267102 Effect1 Dread chance prerequisite"
    );
    assert_eq!(
        core_stacks(run.app.world()),
        0,
        "no injected or pre-earned Core"
    );
    cast_and_finish(&mut run, actor, 104316);
    let world = run.app.world_mut();
    let dogs: Vec<_> = world
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == actor
                && npc.template_id == 98035
                && matches!(summon.spell_id, 193331 | 193332)
        })
        .map(|(entity, summon, _)| {
            (
                entity,
                summon.spell_id,
                summon.expires_at.expect("native Dread lifetime"),
            )
        })
        .collect();
    assert_eq!(
        dogs.len(),
        2,
        "ordinary104316 must first produce both authored children"
    );
    let expiry = dogs.iter().map(|dog| dog.2).max_by(f64::total_cmp).unwrap();
    while crate::spell_cast::now(run.app.world()) <= expiry {
        run.app.update();
    }
    run.app.update();
    let remaining = dogs
        .iter()
        .filter(|dog| run.app.world().get_entity(dog.0).is_ok())
        .count();
    let earned = core_stacks(run.app.world());
    let path = inputs::directory()
        .unwrap()
        .join("266.dreadstalker-retirement-core.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&json!({
        "policy": "SIMC_DEMONIC_CORE_DREADSTALKER_EXPIRY", "native_driver": 267102,
        "native_effect": 1, "native_chance": chance,
        "dogs": dogs.iter().map(|dog| json!({"entity": dog.0.to_bits(), "root": dog.1, "expiry": dog.2})).collect::<Vec<_>>(),
        "remaining": remaining, "earned_core_stacks": earned,
    })).unwrap()).unwrap();
    assert_eq!(
        remaining, 0,
        "observe actual retirement, not a pending timer"
    );
    assert_eq!(
        earned as usize,
        dogs.len(),
        "each native Dread retirement earns one Core; trace {}",
        path.display()
    );
    for _ in 0..5 {
        run.app.update();
    }
    assert_eq!(
        core_stacks(run.app.world()),
        earned,
        "retired actors cannot grant duplicate Core"
    );
}

#[test]
#[ignore = "map cleanup before native TTL must not notify natural retirement; private spec266 inputs required"]
fn dps_parity_demo_dreadstalker_map_cleanup_does_not_earn_expiry_core() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_authored_guardian_control();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 104316);
    let world = run.app.world_mut();
    let dogs: Vec<_> = world
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(world)
        .filter(|(_, summon, npc)| {
            summon.owner == actor
                && npc.template_id == 98035
                && matches!(summon.spell_id, 193331 | 193332)
        })
        .map(|(entity, summon, _)| (entity, summon.expires_at.unwrap()))
        .collect();
    assert_eq!(dogs.len(), 2, "actual root/children prerequisite");
    assert!(dogs.iter().all(|dog| crate::spell_cast::now(world) < dog.1));
    for dog in &dogs {
        world.entity_mut(dog.0).insert(crate::maps::InMap {
            map_id: 34,
            instance_id: 1,
        });
    }
    run.app.update();
    run.app.update();
    let world = run.app.world();
    assert!(
        crate::spell_cast::targets::is_alive(world, actor),
        "retain the selected living owner"
    );
    assert!(
        dogs.iter().all(|dog| world.get_entity(dog.0).is_err()),
        "observe actual map cleanup"
    );
    assert!(
        dogs.iter().all(|dog| crate::spell_cast::now(world) < dog.1),
        "cleanup occurred before authored expiry"
    );
    assert!(
        !crate::auras::has_aura(world, actor, 264173),
        "map cleanup is not natural expiry"
    );
}

#[test]
#[ignore = "native summon effects override the approved project dispatch; private spec266 inputs required"]
fn dps_parity_demo_native_dreadstalker_effect_overrides_project_dispatch() {
    let _seed = crate::random::seed_scope(70448);
    for native_index in [0, 1] {
        let mut run = prepare_authored_guardian_control();
        let mut store = run
            .app
            .world_mut()
            .resource_mut::<crate::spell_info::SpellStore>();
        // Hypothetical future native data: actual193331 Effect28 supplies all
        // creature/duration/geometry fields. Index1 also retains original Dummy0.
        let mut summon_effect = store.get(193331).unwrap().effects[0].clone();
        assert_eq!(
            summon_effect.effect,
            crate::spell_cast::summon::EFFECT_SUMMON
        );
        summon_effect.index = native_index;
        let mut root = store.get(104316).unwrap().clone();
        root.effects.retain(|effect| effect.index != native_index);
        root.effects.push(summon_effect);
        store.replace_fixture_spell(root);
        let actor = run.actor;
        cast_and_finish(&mut run, actor, 104316);
        let world = run.app.world_mut();
        let roots: Vec<_> = world
            .query::<(
                &crate::spell_cast::summon::SummonedCreature,
                &shared::components::Npc,
            )>()
            .iter(world)
            .filter(|(summon, npc)| summon.owner == actor && npc.template_id == 98035)
            .map(|(summon, _)| summon.spell_id)
            .collect();
        assert_eq!(
            roots,
            [104316],
            "native effect{native_index} must not gain two project actors"
        );
    }
}

fn prepare_manual_guardian_control() -> Run {
    let mut run = prepare(266, 3).unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(run.actor)
        .unwrap()
        .0 = Some(run.targets[0]);
    run
}

fn spawn_native_guardian_in_melee_range(run: &mut Run, spell_id: u32) -> Entity {
    let world = run.app.world_mut();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 28)
        .unwrap();
    let position = *world
        .get::<shared::components::Position>(run.targets[0])
        .unwrap();
    // Exercise native Effect28 through actor AI/combat. Destination42/43 planning
    // requires geometry absent from this fixture; do not invent a collision map.
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: run.actor,
        spell: &spell,
        depth: 0,
        dest: Some(crate::spell_cast::targets::Dest {
            position: Vec3::new(position.x, position.y, position.z),
            orientation: 0.0,
        }),
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::cast_handler(effect.effect).unwrap()(world, &ctx, effect);
    world
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == spell_id)
        .map(|(entity, _)| entity)
        .expect("real native effect28 must produce its actual guardian")
}

fn read_guardian_swings(world: &World, actor: Entity) -> Vec<(f64, u64, f32)> {
    use shared::protocol::CombatEventType;
    world
        .resource::<crate::combat::CombatLog>()
        .range(0.0, f64::MAX)
        .into_iter()
        .filter(|entry| entry.event.attacker == actor.to_bits() && entry.event.spell_id == 0)
        .filter(|entry| {
            matches!(
                entry.event.event_type,
                CombatEventType::MeleeDamage
                    | CombatEventType::CriticalHit
                    | CombatEventType::Miss
                    | CombatEventType::Dodge
                    | CombatEventType::Parry
                    | CombatEventType::Block
            )
        })
        .map(|entry| (entry.timestamp, entry.event.target, entry.event.amount))
        .collect()
}

#[test]
#[ignore = "real90 Dreadstalker melee/cadence/retarget/stop boundary; private spec266 inputs required"]
fn dps_parity_demo_native_dreadstalker_melee_tracks_owner_without_resetting_clock() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_manual_guardian_control();
    let guardian = spawn_native_guardian_in_melee_range(&mut run, 193331);
    for _ in 0..100 {
        run.app.update();
    }
    let first = read_guardian_swings(run.app.world(), guardian);
    assert_eq!(
        first.len(),
        3,
        "real Dreadstalker must start native melee and swing every TC2000ms: {first:?}"
    );
    assert!(
        first.iter().any(|(_, _, damage)| *damage > 0.0),
        "native inherited owner power must produce actual melee damage"
    );
    assert!(
        first
            .iter()
            .all(|(_, victim, _)| *victim == run.targets[0].to_bits())
    );
    for pair in first.windows(2) {
        assert!(
            (pair[1].0 - pair[0].0 - 2.0).abs() <= 0.051,
            "TC Guardian base swing clock: {first:?}"
        );
    }
    run.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(run.actor)
        .unwrap()
        .0 = Some(run.targets[1]);
    for _ in 0..40 {
        run.app.update();
    }
    let retargeted = read_guardian_swings(run.app.world(), guardian);
    let next = retargeted
        .get(first.len())
        .expect("real guardian must pick up the owner's new target");
    assert_eq!(next.1, run.targets[1].to_bits());
    assert!(
        (next.0 - first.last().unwrap().0 - 2.0).abs() <= 0.051,
        "retarget keeps the running swing clock: {retargeted:?}"
    );
    run.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(run.actor)
        .unwrap()
        .0 = None;
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        read_guardian_swings(run.app.world(), guardian),
        retargeted,
        "clearing owner target must stop real guardian swings"
    );
}

#[test]
#[ignore = "real90 caster-Imp melee exclusion control; private spec266 inputs required"]
fn dps_parity_demo_native_caster_imp_keeps_firebolt_without_guardian_melee() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_manual_guardian_control();
    let guardian = spawn_native_guardian_in_melee_range(&mut run, 104317);
    for _ in 0..200 {
        run.app.update();
    }
    assert!(
        read_guardian_swings(run.app.world(), guardian).is_empty(),
        "caster Imps do not acquire guardian melee"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(guardian.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "excluding caster melee must preserve real native Firebolt damage"
    );
}

fn advance_until_cast_finished(run: &mut Run, unit: Entity) {
    for _ in 0..400 {
        if run
            .app
            .world()
            .get::<shared::casting::CastState>(unit)
            .is_none()
        {
            return;
        }
        run.app.update();
    }
    panic!("native resource-control cast never finished");
}

fn cast_and_finish(run: &mut Run, unit: Entity, spell: u32) {
    let target = run.targets[0];
    for _ in 0..400 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            unit,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                advance_until_cast_finished(run, unit);
                return;
            }
            Err(
                crate::spell_cast::CastFailure::InProgress
                | crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NotEnoughPower(_),
            ) => run.app.update(),
            Err(error) => panic!("native resource-control spell{spell}: {error:?}"),
        }
    }
    panic!("native resource-control spell{spell} never became ready");
}

#[test]
#[ignore = "native class-follower and Felguard regen control; requires private DB and spec266 profile"]
fn dps_parity_demo_native_follower_and_felguard_keep_resource_regeneration() {
    use shared::components::{Mana, Position, PowerType};
    let mut run = prepare(266, 1).unwrap();
    let controller = fixture_player(&run);
    run.app.world_mut().entity_mut(controller).insert(Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    for _ in 0..200 {
        run.app.update();
    }
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::pet_control::PetCharm>(pet)
        .unwrap()
        .react = shared::protocol::REACT_PASSIVE;
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, None);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    // Exclusive fixture controller prevents unrelated casts from spending the
    // pools while ordinary native spells/regeneration exercise both real roles.
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    let actor = run.actor;
    cast_and_finish(&mut run, actor, 686);
    let before = run.app.world().get::<Mana>(actor).unwrap().current;
    assert!(before < run.app.world().get::<Mana>(actor).unwrap().max);
    for _ in 0..20 {
        run.app.update();
    }
    let after = run.app.world().get::<Mana>(actor).unwrap().current;
    assert!(
        after > before,
        "real class-follower mana must regenerate after native Shadow Bolt"
    );
    // This control isolates regeneration, not movement: place the real pet at
    // its real target before spending Energy through native Legion Strike.
    let target_position = *run.app.world().get::<Position>(run.targets[0]).unwrap();
    run.app.world_mut().entity_mut(pet).insert(target_position);
    cast_and_finish(&mut run, pet, 30213);
    let (before, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Energy).unwrap();
    assert!(
        before < maximum,
        "native Legion Strike must spend Felguard Energy"
    );
    let rate = run
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Energy)
        .unwrap()
        .regen_combat;
    for _ in 0..20 {
        run.app.update();
    }
    let after = crate::spell_cast::powers::power_value(run.app.world(), pet, PowerType::Energy)
        .unwrap()
        .0;
    assert!(
        (after - before - rate).abs() < 0.01,
        "real controlled Felguard keeps its native per-second Energy rate: before{before}, after{after}, rate{rate}"
    );
}

fn summon_count(world: &World, owner: Entity) -> usize {
    world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(owner.to_bits())
                && event.spell_id == Some(30146)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count()
}

#[test]
#[ignore = "native Demo OOC pet preparation and first pull; requires private DB and spec266 profile"]
fn dps_parity_demo_native_prepares_felguard_once_and_attacks_first_pull() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare(266, 1).unwrap();
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_none());
    for _ in 0..200 {
        run.app.update();
    }
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("native kit must finish Felguard preparation out of combat");
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        17252,
        "authentic Felguard, not a substituted guardian"
    );
    assert_eq!(summon_count(run.app.world(), run.actor), 1);
    engage(&mut run);
    for _ in 0..400 {
        run.app.update();
    }
    assert_eq!(
        crate::hunter_pet::summoned_pet(run.app.world(), run.actor),
        Some(pet)
    );
    assert_eq!(
        summon_count(run.app.world(), run.actor),
        0,
        "engage clears precombat logs; native kit must not resummon its existing pet"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage: i64 = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.target == Some(run.targets[0].to_bits())
                && event.source == Some(run.actor.to_bits())
        })
        .map(|event| i64::from(event.amount))
        .sum();
    assert!(
        damage > 0,
        "prepared native Demo must damage the first pull"
    );
}
