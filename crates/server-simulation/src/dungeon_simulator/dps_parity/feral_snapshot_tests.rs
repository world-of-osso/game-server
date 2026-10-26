//! Literal periodic operands must read native frozen values and fresh paid-point metadata.
use super::feral_growth_tests::cast_native_spell_after_regeneration as cast_native;
use super::*;
use crate::simc_apl_native::{
    Bindings, DotRefresh, DotRefreshPolicy, Encounter, FightStyle, NativeFacts, native_dot_spell,
};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

fn bindings() -> Bindings {
    Bindings {
        actions: [("rake".into(), 1822), ("rip".into(), 1079)].into(),
        buffs: [("rake".into(), 155722), ("rip".into(), 1079)].into(),
        dot_refresh: ["rake", "rip"]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    DotRefresh {
                        resolve: native_dot_spell,
                        policy: DotRefreshPolicy::Pandemic,
                    },
                )
            })
            .collect(),
        ..Default::default()
    }
}

fn encounter(run: &Run) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_periodic_facts_preserve_snapshot_until_refresh() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5215, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let bindings = bindings();
    let encounter = encounter(&run);
    let action = Action {
        name: "rake".into(),
        source_row: None,
        target: Some(target.to_bits()),
        options: Default::default(),
    };
    let read = |run: &Run, name: &str| {
        NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1)
            .with_targets(Some(target), &run.targets)
            .value_for(&action, name)
            .unwrap()
    };
    assert_eq!(read(&run, "refreshable"), 1.0);
    assert_eq!(read(&run, "pmultiplier"), 0.0);
    let stealthed = read(&run, "persistent_multiplier");
    assert!(stealthed > 1.0, "native Prowl improves the next Rake");
    cast_native(&mut run, 1822);
    assert!(!crate::auras::has_aura(run.app.world(), actor, 5215));
    assert_eq!(read(&run, "pmultiplier"), stealthed);
    let ordinary = read(&run, "persistent_multiplier");
    assert!(
        ordinary < stealthed,
        "live state cannot rewrite a stored bleed snapshot"
    );
    cast_native(&mut run, 1822);
    assert_eq!(read(&run, "pmultiplier"), ordinary);
    assert_eq!(read(&run, "action.rake.pmultiplier"), ordinary);
}

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_rip_refresh_reads_fresh_earned_combo_duration() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let combo = |run: &Run| {
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::ComboPoints)
            .unwrap()
            .0
    };
    while combo(&run) < 5.0 {
        cast_native(&mut run, 5221);
    }
    cast_native(&mut run, 1079);
    let target = run.targets[0];
    let duration = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == actor && aura.spell_id == 1079)
        .unwrap()
        .duration_ms;
    while combo(&run) < 5.0 {
        cast_native(&mut run, 5221);
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == actor && aura.spell_id == 1079)
        .unwrap();
    let remains = aura.expires_at.unwrap() - crate::spell_cast::now(run.app.world());
    let frames = ((remains - f64::from(duration) / 1000.0 * 0.20).max(0.0) / 0.05).ceil() as usize;
    for _ in 0..frames {
        run.app.update();
    }
    let bindings = bindings();
    let facts = NativeFacts::new(run.app.world(), actor, &bindings, encounter(&run), 1)
        .with_targets(Some(target), &run.targets);
    assert_eq!(
        facts.value("dot.rip.refreshable").unwrap(),
        1.0,
        "twenty percent of five-CP Rip is inside the next five-CP pandemic window"
    );
    assert_eq!(combo(&run), 5.0, "a metadata query cannot pay combo points");
}
