//! The actual equipped Lightwood driver must produce its native damage payload.
use super::*;
use crate::auras::{ActiveAuras, AuraSource};
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native259 and260 player profiles, traits and world DB"]
fn equipped_lightwood_deals_native_proc_damage_for_both_rogues() {
    let mut missing = Vec::new();
    for spec in [259, 260] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(spec, 1).unwrap();
        let driver = run
            .app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1296732 && aura.caster == run.actor)
            .expect("the actual profile equips the Lightwood driver");
        assert!(matches!(driver.source, AuraSource::Equipment(_)));
        assert!(driver.cast_item_level.is_some());
        assert!(
            driver
                .effects
                .iter()
                .any(|effect| effect.index == 0 && effect.amount > 0.0),
            "native item-level-scaled damage budget must be positive"
        );
        engage(&mut run);
        let (seconds, _, _) = advance_combat(&mut run).unwrap();
        assert!(seconds >= 299.0);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(1296735)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        println!("Lightwood spec={spec}: positive native hits={hits}");
        if hits == 0 {
            missing.push(spec);
        }
    }
    // SimC76f8118 unique_gear_midnight.cpp5219–5230: the actual equipped
    // driver rolls its native proc and its linked action uses the driver budget.
    assert!(
        missing.is_empty(),
        "equipped Lightwood must deal native proc damage: {missing:?}"
    );
}
