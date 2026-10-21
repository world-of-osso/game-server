//! TC a352b1fa spell_dk.cpp1328-1358: only Gorefiend's Grasp E1 is hooked.
//! Native E0 pull remains unimplemented; no generic77 promotion.
use super::test_harness::{Harness, ObservedTriggeredCasts, harness, world_db};

const GRASP: u32 = 108199;
const TALENT: u32 = 454822;
const DEBUFF: u32 = 454824;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[GRASP, TALENT, DEBUFF], 6).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app.insert_resource(ObservedTriggeredCasts::default());
    h.advance(3);
    h
}

fn cast_grasp(h: &mut Harness) {
    super::creature_cast(h.app.world_mut(), h.caster, GRASP, Some(h.npc), true).unwrap();
    h.advance(3);
}

#[test]
#[ignore = "RED: census-e77 DK Subduing Grasp effect1 not bound"]
fn census_e77_dk_own_native_talent_applies_owned_subduing_grasp() {
    let mut h = fixture();
    super::creature_cast(h.app.world_mut(), h.caster, TALENT, Some(h.caster), true).unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, TALENT));
    cast_grasp(&mut h);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DEBUFF)
        .expect("native108199 E1 must cast454824 with own454822");
    assert_eq!(aura.caster, h.caster);
    assert!(aura.effects.iter().any(|effect| effect.aura_type == 269));
    let launches = &h.app.world().resource::<ObservedTriggeredCasts>().0;
    assert!(launches.contains(&(h.caster, DEBUFF, Some(h.npc))));
}

#[test]
#[ignore = "RED batch control: no talent must not apply debuff"]
fn census_e77_dk_no_talent_does_not_apply_subduing_grasp() {
    let mut h = fixture();
    cast_grasp(&mut h);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, DEBUFF));
    assert!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .is_empty()
    );
}

#[test]
#[ignore = "RED batch control: foreign talent must not apply debuff"]
fn census_e77_dk_foreign_talent_does_not_apply_subduing_grasp() {
    let mut h = fixture();
    super::creature_cast(h.app.world_mut(), h.npc, TALENT, Some(h.npc), true).unwrap();
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, TALENT));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, TALENT));
    cast_grasp(&mut h);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, DEBUFF));
    assert!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .is_empty()
    );
}
