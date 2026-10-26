//! Attribute native rating changes to actual equipped sources before parity scoring.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_idle_setup_cannot_roll_temporary_equipment_stat_procs() {
    let _seed = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(255).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = spawn_reference(profile, &traits, 1, identity).unwrap();
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    for frame in 0..6 {
        if frame > 0 {
            run.app.update();
        }
        log_rating_sources(&run, &format!("idle_frame_{frame}"));
    }
    let idle_procs: Vec<_> = [1307910, 1241762, 1287774]
        .into_iter()
        .filter(|&spell| crate::auras::has_aura(run.app.world(), run.actor, spell))
        .collect();
    let idle_crit = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .ratings
        .crit;
    assert_eq!(idle_crit, 606, "actual permanent gear and Serrated Tips crit");
    preparation::consume_profile_food(&mut run).unwrap();
    log_rating_sources(&run, "after_food");
    let driver = survival_rotation::prepare_driver(&mut run).unwrap();
    log_rating_sources(&run, "after_precombat");
    eprintln!("SURVIVAL_SOURCE_SNAPSHOT {}", driver.report());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for event in events
        .iter()
        .filter(|event| event.kind != CombatLogKind::AuraApplied)
    {
        eprintln!("SURVIVAL_SETUP_EVENT {event:?}");
    }
    assert!(
        idle_procs.is_empty(),
        "idle setup cannot earn temporary item-stat procs before an actual action: {idle_procs:?}"
    );
}

fn log_rating_sources(run: &Run, stage: &str) {
    let world = run.app.world();
    let stats = world.get::<crate::stats::CharacterStats>(run.actor);
    let mods = world.get::<crate::stats::StatModifiers>(run.actor);
    eprintln!(
        "SURVIVAL_RATING_INPUT stage={stage} t={} ratings={:?} aura_flat={:?} rating_pct={:?}",
        crate::spell_cast::now(world),
        stats.map(|stats| stats.ratings),
        mods.map(|mods| mods.ratings_flat),
        mods.map(|mods| &mods.rating_percent_mods)
    );
    if let Some(auras) = world.get::<crate::auras::ActiveAuras>(run.actor) {
        for aura in auras.iter() {
            let effects: Vec<_> = aura
                .effects
                .iter()
                .filter(|effect| effect.aura_type == 189)
                .map(|effect| (effect.index, effect.amount, effect.misc_values[0]))
                .collect();
            if !effects.is_empty() {
                eprintln!(
                    "SURVIVAL_RATING_AURA stage={stage} spell={} item_level={:?} effects={effects:?}",
                    aura.spell_id, aura.cast_item_level
                );
            }
        }
    }
}
