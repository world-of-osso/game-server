//! Alias matching for the Devastation consumer of the shared native history fact.
use super::test_harness::*;
use crate::simc_apl::Facts;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};

#[test]
fn prev_gcd_deep_breath_alias_matches_native_base_and_selected_override() {
    let mut h = harness(Vec::new(), &[357210, 433874], 13).unwrap();
    let mut traits = crate::class_progression::TraitLoadout::default();
    traits.overrides.push((357210, 433874));
    h.app.world_mut().entity_mut(h.caster).insert(traits);
    let bindings = Bindings {
        actions: [("deep_breath".into(), 357210)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    for id in [357210, 433874] {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(id)
            .unwrap()
            .clone();
        let mut history = super::SpellHistory::default();
        history.record_successful_cast(&spell, super::now(h.app.world()));
        h.app.world_mut().entity_mut(h.caster).insert(history);
        let facts = NativeFacts::new(h.app.world(), h.caster, &bindings, encounter, 1);
        assert_eq!(
            facts.value("prev_gcd.1.deep_breath").unwrap(),
            1.0,
            "native action{id} retains the deep_breath APL alias"
        );
    }
}
