//! Real cast/effect completion for an adapter-selected native precombat plan.
use super::{literal_rotation, preparation, prepare_literal};
use bevy::prelude::*;
use literal_rotation::execution::{NativeAction, NativeComposite, Prepared};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

const COATINGS: [u32; 4] = [2823, 3408, 381664, 381637];
const NAMES: [&str; 4] = [
    "deadly_poison",
    "crippling_poison",
    "amplifying_poison",
    "atrophic_poison",
];

#[derive(Resource)]
struct ObservedCoatingBars {
    actor: Entity,
    durations: BTreeMap<u32, f32>,
}

fn observe_coating_bars(
    casts: Query<&shared::casting::CastState>,
    mut observed: ResMut<ObservedCoatingBars>,
) {
    let Ok(cast) = casts.get(observed.actor) else {
        return;
    };
    if COATINGS.contains(&cast.spell_id) {
        observed
            .durations
            .entry(cast.spell_id)
            .or_insert(cast.duration);
    }
}

#[test]
#[ignore = "native259 composite lifecycle RED; requires current private inputs and player identity"]
fn literal_precombat_composite_waits_for_native_coatings_and_effects() {
    let _seed = crate::random::seed_scope(70448);
    assert_eq!(std::env::var("DPS_PARITY_ACTOR").unwrap(), "player");
    let mut run = prepare_literal(259, 1).unwrap();
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(
        known.contains(381801),
        "selected Dragon-Tempered Blades required"
    );
    assert!(known.contains(381637), "selected Atrophic Poison required");
    let equipment = literal_rotation::equipment::Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let steps: Vec<_> = NAMES
        .into_iter()
        .zip(COATINGS)
        .map(|(name, spell)| NativeAction {
            name,
            spell,
            talent: None,
            self_target: true,
        })
        .collect();
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &[],
        &equipment,
        steps.iter().copied(),
        true,
    )
    .unwrap();
    for name in NAMES {
        let action = server::simc_apl::Action {
            name: name.into(),
            target: None,
            options: Default::default(),
        };
        assert!(
            prepared.execution(&action, &equipment).unwrap().is_some(),
            "native admission premise failed for {name}; not a composite RED"
        );
    }
    let prepared = prepared
        .with_composites(
            run.app.world_mut(),
            run.actor,
            &[],
            [NativeComposite {
                name: "apply_poison",
                steps: &steps,
            }],
        )
        .unwrap();
    let program = server::simc_apl::Program::parse(&[
        ("precombat", "apply_poison"),
        ("precombat", "snapshot_stats"),
    ])
    .unwrap();
    let mut driver = literal_rotation::Driver::new(program);
    let facts = BTreeMap::<String, f64>::new();
    let (action, plan) = driver
        .select("precombat", &facts, &prepared, &equipment)
        .unwrap()
        .expect("all native coatings admitted but composite selection absent");
    run.app.insert_resource(ObservedCoatingBars {
        actor: run.actor,
        durations: BTreeMap::new(),
    });
    run.app.add_systems(Update, observe_coating_bars);
    let before = crate::spell_cast::now(run.app.world());
    preparation::execute_precombat_plan(&mut run, &mut driver, action, plan)
        .expect("selected composite must complete through the native App lifecycle");
    let elapsed = crate::spell_cast::now(run.app.world()) - before;
    let bars = &run.app.world().resource::<ObservedCoatingBars>().durations;
    assert_eq!(
        bars.len(),
        COATINGS.len(),
        "every coating needs a real cast bar"
    );
    assert!(bars.values().all(|duration| *duration > 0.0));
    let cast_seconds: f64 = bars.values().map(|duration| f64::from(*duration)).sum();
    assert!(
        elapsed + 0.001 >= cast_seconds,
        "casts must not overlap or finish instantly"
    );
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_none()
    );
    let history = run
        .app
        .world()
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .unwrap();
    assert!(!history.has_active_gcd(crate::spell_cast::history_now(run.app.world())));
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id.filter(|spell| COATINGS.contains(spell)))
        .collect();
    assert_eq!(
        casts, COATINGS,
        "actual coating completions must retain source order"
    );
    let now = crate::spell_cast::now(run.app.world());
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap();
    for spell in COATINGS {
        assert!(
            auras.iter().any(|aura| aura.spell_id == spell
                && aura.caster == run.actor
                && aura.expires_at.is_none_or(|at| at > now)),
            "owned coating {spell} missing at pull boundary"
        );
    }
    for &target in &run.targets {
        assert!(!crate::combat_refs::has_active_reference_to(
            run.app.world(),
            run.actor,
            target
        ));
    }
    let (action, execution) = driver
        .select("precombat", &facts, &prepared, &equipment)
        .unwrap()
        .expect("the next source row must remain available after the composite");
    preparation::execute_precombat_plan(&mut run, &mut driver, action, execution).unwrap();
    assert!(
        driver.snapshot.is_some(),
        "post-composite snapshot must execute before pull"
    );
}
