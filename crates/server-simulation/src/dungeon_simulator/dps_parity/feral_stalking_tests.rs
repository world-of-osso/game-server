//! SimC76f8118 sc_druid.cpp4879/11313–11316: TF earns maximum Stalking stacks.
use super::*;

#[test]
#[ignore = "requires exact spec103 profile/traits and native private DB"]
fn feral_native_tigers_fury_earns_native_maximum_stalking_predator_stacks() {
    let mut run = prepare_literal(103, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1263660),
        "the actual profile selects Unseen Predator3"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 1263962),
        "ordinary Cat Form cannot earn Tiger's Fury's Stalking Predator uses"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(5217, actor, Some(target)),
    )
    .expect("ordinary selected native Tiger's Fury");
    run.app.update();
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1263962 && aura.caster == actor)
        })
        .map(|aura| aura.stacks);
    assert_eq!(
        stacks,
        Some(2),
        "real Tiger's Fury grants native Stalking Predator maximum stacks"
    );
}
