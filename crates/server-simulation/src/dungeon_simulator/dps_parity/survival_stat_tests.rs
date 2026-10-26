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
        let stats = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap();
        let mods = run
            .app
            .world()
            .get::<crate::stats::StatModifiers>(run.actor);
        eprintln!(
            "SURVIVAL_RATING_INPUT frame={frame} t={} ratings={:?} aura_flat={:?}",
            crate::spell_cast::now(run.app.world()),
            stats.ratings,
            mods.map(|mods| mods.ratings_flat)
        );
        if let Some(auras) = run.app.world().get::<crate::auras::ActiveAuras>(run.actor) {
            for aura in auras.iter() {
                let rating_effects: Vec<_> = aura
                    .effects
                    .iter()
                    .filter(|effect| effect.aura_type == 189)
                    .map(|effect| (effect.index, effect.amount, effect.misc_values[0]))
                    .collect();
                if !rating_effects.is_empty() {
                    eprintln!(
                        "SURVIVAL_RATING_AURA frame={frame} spell={} item_level={:?} effects={rating_effects:?}",
                        aura.spell_id, aura.cast_item_level
                    );
                }
            }
        }
    }
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
    for spell in [1307910, 1241762, 1287774] {
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, spell),
            "idle fixture initialization cannot earn temporary item-stat proc{spell} before any actual action"
        );
    }
}
