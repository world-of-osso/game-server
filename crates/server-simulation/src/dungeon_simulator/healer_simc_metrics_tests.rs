use super::Recorder;
use bevy::prelude::{Real, Time, World};
use shared::casting::CastState;
use shared::components::Health;

#[test]
#[ignore = "requires native spell data"]
fn healer_report_counts_only_consumed_owned_shields() {
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[133, 17], 5).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let other_healer = crate::spell_cast::test_harness::spawn_player(&mut h.app, 5, 2.0);
    h.app.insert_resource(super::super::healer_intake::Capture {
        members: vec![h.caster, other_healer],
        sources: [(h.npc, (6, 6))].into(),
        ..Default::default()
    });
    let recorder = Recorder::start(h.app.world(), h.caster, h.caster, vec![h.caster]);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        recorder.report(h.app.world(), 1)["consumed_absorbs_by_spell"],
        serde_json::json!({})
    );
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let before = h.health(h.caster);
    crate::spell_cast::effects::deal_damage_shield(
        h.app.world_mut(),
        &crate::spell_cast::effects::EffectCtx {
            caster: h.npc,
            spell: &spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.npc,
        h.caster,
        200.0,
    );
    assert_eq!(h.health(h.caster), before);
    assert_eq!(
        recorder.report(h.app.world(), 1)["consumed_absorbs_by_spell"]["17"],
        200.0
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: other_healer,
            target: other_healer,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let before = h.health(other_healer);
    crate::spell_cast::effects::deal_damage_shield(
        h.app.world_mut(),
        &crate::spell_cast::effects::EffectCtx {
            caster: h.npc,
            spell: &spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.npc,
        other_healer,
        100.0,
    );
    assert_eq!(h.health(other_healer), before);
    assert_eq!(
        recorder.report(h.app.world(), 1)["consumed_absorbs_by_spell"]["17"],
        200.0,
        "the other healer's consumed shield must not be attributed to this healer"
    );
}

#[test]
#[ignore = "requires the MID2 Holy player and canonical world database"]
fn idle_gcd_excludes_native_global_cooldown_after_an_instant_cast() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 65).unwrap();
    let mut recorder = Recorder::start(run.app.world(), healer, run.actor, vec![healer]);
    recorder.observe_idle_gcd(run.app.world());
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 20473, Some(healer), false)
        .unwrap();
    assert!(run.app.world().get::<CastState>(healer).is_none());
    for _ in 0..19 {
        recorder.observe_idle_gcd(run.app.world());
    }
    assert_eq!(recorder.report(run.app.world(), 1)["idle_gcd_pct"], 5.0);
}

#[test]
fn idle_gcd_counts_only_living_frames_without_cast_or_channel() {
    let mut world = World::new();
    world.insert_resource(Time::<Real>::default());
    world.insert_resource(crate::spell_cast::test_harness::ObservedCombatEvents::default());
    let healer = world
        .spawn(Health {
            current: 100.0,
            max: 100.0,
        })
        .id();
    let mut recorder = Recorder::start(&world, healer, healer, vec![healer]);
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world
        .entity_mut(healer)
        .insert(CastState::normal(1, 0, 2.0, false));
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world
        .entity_mut(healer)
        .insert(CastState::channel(2, 0, 2.0, 1.0, false));
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    world.entity_mut(healer).remove::<CastState>();
    world.get_mut::<Health>(healer).unwrap().current = 0.0;
    for _ in 0..5 {
        recorder.observe_idle_gcd(&world);
    }
    assert_eq!(recorder.report(&world, 1)["idle_gcd_pct"], 25.0);
}
