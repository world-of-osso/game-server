//! Alias matching for the Devastation consumer of the shared native history fact.
use super::test_harness::*;
use crate::simc_apl::Facts;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};

#[test]
fn class_follower_successful_spell_populates_program_bounded_history() {
    let mut h = harness(Vec::new(), &[133], 8).unwrap();
    let program =
        crate::simc_apl::Program::parse(&[("default", "fireball,if=prev_gcd.1.fireball")]).unwrap();
    crate::simc_apl::gcd_history::initialize(h.app.world_mut(), h.caster, &program);
    h.cast_and_deliver(133, Some(h.npc));
    h.advance(100);
    let history = h
        .app
        .world()
        .get::<crate::simc_apl::gcd_history::GcdHistory>(h.caster)
        .unwrap();
    assert_eq!(history.previous(1).unwrap(), Some(133));
    assert!(history.previous(2).is_err());
}

#[test]
fn class_follower_prev_gcd_deep_breath_alias_matches_native_base_and_selected_override() {
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
        let at = super::now(h.app.world());
        history.record_successful_cast(&spell, at);
        history.record_fragment_pickup(at + 1.0);
        assert_eq!(history.last_successful_cast(), Some(id));
        assert_eq!(history.last_successful_cast_at(id), Some(at));
        let mut gcd_history = crate::simc_apl::gcd_history::GcdHistory::new(1);
        gcd_history.record(spell.id);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert((history, gcd_history));
        let facts = NativeFacts::new(h.app.world(), h.caster, &bindings, encounter, 1);
        assert_eq!(
            facts.value("prev_gcd.1.deep_breath").unwrap(),
            1.0,
            "native action{id} retains the deep_breath APL alias"
        );
    }
}
