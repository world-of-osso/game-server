//! Frozen source pressure, real native follower decisions and recipient mitigation.
use super::healer_intake::{Hit, HitKind, Intake};
use bevy::prelude::*;
use serde_json::{Value, json};

#[derive(Resource, Default)]
struct PendingHits {
    hits: Vec<(Entity, Entity, Hit)>,
    error: Option<String>,
    manual_raw_receipts: usize,
    replayed_hits: usize,
    non_unit_hits: usize,
}

fn deliver_hits(world: &mut World) {
    let hits = std::mem::take(&mut world.resource_mut::<PendingHits>().hits);
    for (source, target, hit) in hits {
        let before = world.resource::<super::healer_intake::Capture>().hits.len();
        let scale = crate::level_scaling::damage_to_hp_scale(world, source, target);
        if !scale.is_finite() || scale <= 0.0 {
            world.resource_mut::<PendingHits>().error =
                Some("NATIVE_REPLAY_LEVEL_SCALING_FACTOR_INVALID".into());
            return;
        }
        world
            .resource_mut::<super::healer_intake::Capture>()
            .active_hit_scale = Some((target, scale));
        {
            let mut pending = world.resource_mut::<PendingHits>();
            pending.replayed_hits += 1;
            pending.non_unit_hits += usize::from(scale != 1.0);
        }
        let result = match hit.kind {
            HitKind::Melee { .. } => crate::combat::replay_healer_hit(world, source, target, &hit),
            HitKind::Spell { .. } => {
                crate::spell_cast::effects::replay_healer_hit(world, source, target, &hit)
            }
        };
        world
            .resource_mut::<super::healer_intake::Capture>()
            .active_hit_scale = None;
        let added = world.resource::<super::healer_intake::Capture>().hits.len() - before;
        world.resource_mut::<PendingHits>().manual_raw_receipts += added;
        if let Err(error) = result {
            world.resource_mut::<PendingHits>().error = Some(error);
            return;
        }
    }
}

pub(super) fn recorded_combat_seconds(intake: &Intake) -> f64 {
    let mut frame = 0;
    let mut ticks = 0_u32;
    for tick in 1..=intake.end_tick {
        while frame + 1 < intake.frames.len() && intake.frames[frame + 1].tick <= tick {
            frame += 1;
        }
        ticks += u32::from(intake.frames[frame].in_combat);
    }
    f64::from(ticks) * 0.05
}

fn level_scaling_report(world: &World, healer: Entity, recorded: &[Hit]) -> Value {
    let usage = world
        .resource::<super::healer_intake::Capture>()
        .consumed_absorbs
        .get(&healer)
        .copied()
        .unwrap_or_default();
    let pending = world.resource::<PendingHits>();
    let non_unit = super::healer_intake::non_unit_hits(recorded);
    json!({"recorded_hits":recorded.len(),"recorded_unit_hits":recorded.len()-non_unit,
        "recorded_non_unit_hits":non_unit,"replayed_hits":pending.replayed_hits,"replayed_non_unit_hits":pending.non_unit_hits,
        "absorbed_points_on_non_unit_hits":usage.non_unit,"absorbed_points_unknown_scale":usage.unknown_scale,
        "absorbed_points_non_unit_share":(usage.total>0.0).then(||usage.non_unit/usage.total),
        "gap_moot_for_intake":!recorded.is_empty() && non_unit==0 && pending.non_unit_hits==0 && usage.unknown_scale==0.0,
        "factor_definition":"native damage multiplier / victim health multiplier, before integer rounding",
        "absorb_convention":"native consumed shield points, not normalized prevented HP"})
}

#[test]
#[ignore = "RED: non-unit consumed-point attribution missing"]
fn healer_benchmark_counts_nonunit_hit_absorbed_point_share() {
    let _rng = crate::random::seed_scope(1);
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_copy(&mut app);
    let actors = super::runtime::spawn_population(&mut app, copy);
    let healer = crate::spell_cast::test_harness::spawn_player(&mut app, 5, 0.0);
    app.world_mut()
        .entity_mut(healer)
        .insert((shared::components::UnitLevel(10), copy));
    let (&guid, &source) = actors
        .iter()
        .find(|&(_, &unit)| {
            let factor = crate::level_scaling::damage_to_hp_scale(app.world(), unit, healer);
            factor.is_finite()
                && factor > 0.0
                && factor != 1.0
                && crate::spell_cast::targets::relation_matches(
                    app.world(),
                    unit,
                    healer,
                    crate::spell_cast::targets::Relation::Enemy,
                )
        })
        .expect("real Stockade10..30 curve against level10 fixture");
    let graph = &app.world().resource::<super::Space>().0;
    let pack = graph
        .packs
        .iter()
        .find(|pack| pack.spawns.iter().any(|spawn| spawn.guid == guid))
        .unwrap()
        .id;
    app.insert_resource(super::healer_intake::Capture {
        members: vec![healer],
        sources: [(source, (guid, pack))].into(),
        ..Default::default()
    });
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: healer,
            target: healer,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let attacker = crate::level_scaling::UnitScaling::of(app.world(), source);
    let victim = crate::level_scaling::UnitScaling::of(app.world(), healer);
    let difference =
        i32::from(victim.level_against(&attacker)) - i32::from(attacker.level_against(&victim));
    super::healer_intake::record(
        app.world_mut(),
        source,
        healer,
        1,
        50.0,
        HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: difference,
            critical_multiplier: 1.0,
            weapon_speed: crate::combat::AUTO_ATTACK_INTERVAL,
        },
    );
    let recorded = app
        .world()
        .resource::<super::healer_intake::Capture>()
        .hits
        .clone();
    // Persisted intake enters a fresh observation context, not the recording hit's tag.
    app.insert_resource(super::healer_intake::Capture {
        members: vec![healer],
        sources: [(source, (guid, pack))].into(),
        ..Default::default()
    });
    app.insert_resource(PendingHits {
        hits: vec![(source, healer, recorded[0].clone())],
        ..Default::default()
    });
    deliver_hits(app.world_mut());
    assert!(app.world().resource::<PendingHits>().error.is_none());
    let summary = level_scaling_report(app.world(), healer, &recorded);
    assert_eq!(summary["recorded_non_unit_hits"], 1);
    assert_eq!(summary["replayed_non_unit_hits"], 1);
    assert_eq!(summary["absorbed_points_on_non_unit_hits"], 50.0);
    assert_eq!(summary["absorbed_points_non_unit_share"], 1.0);
    assert_eq!(summary["gap_moot_for_intake"], false);
}

pub(super) fn replay(_intake: &Intake, _spec: u32) -> Result<Value, String> {
    Err("RAW_INTAKE_REPLAY_NOT_IMPLEMENTED".into())
}
