//! Diabolist ritual → Demonic Art → demon → Ruination/Infernal Bolt on the real 266 follower.
use super::*;
use shared::protocol::CombatLogKind;

const RUINATION_BUFF: u32 = 433885;
const INFERNAL_BOLT_BUFF: u32 = 433891;

/// Real follower with three Shadow Bolt shards, engaged with its dummy.
fn prepare_funded() -> Run {
    let mut run = prepare_literal(266, 1).unwrap();
    for _ in 0..3 {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            686,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    engage(&mut run);
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    run
}

fn cast(run: &mut Run, spell: u32) {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, spell, Some(run.targets[0]), false)
        .unwrap_or_else(|failure| panic!("native {spell} must cast: {failure:?}"));
    preparation::finish_pending_setup(run).unwrap();
}

fn grant_art(run: &mut Run, art: u32) {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, art, Some(run.actor), true)
        .expect("native Demonic Art must apply");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, art));
}

fn events(run: &Run, kind: CombatLogKind, spell: u32) -> Vec<shared::protocol::CombatLogEvent> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits()) && event.spell_id == Some(spell) && event.kind == kind
        })
        .cloned()
        .collect()
}

fn update(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private266 inputs; natural Diabolic Ritual earns its real Art"]
fn demonology_expired_overlord_ritual_earns_demonic_art() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(266, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 428514));
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 431944, Some(actor), true)
        .expect("native Overlord ritual must apply");
    assert!(crate::auras::has_aura(run.app.world(), actor, 431944));
    for tick in 0..420 {
        if tick % 60 == 0 {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 686, Some(target), false)
                .expect("real Shadow Bolt keeps dummy combat active");
        }
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0)
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 431944));
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 428524),
        "natural Overlord ritual expiry must earn its native Demonic Art"
    );
}

#[test]
#[ignore = "requires private266 inputs; Demonic Art consumed by a paid Hand summons the Pit Lord"]
fn demonology_art_hand_summons_pit_lord_and_grants_ruination() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    grant_art(&mut run, 432795);
    cast(&mut run, 105174);
    update(&mut run, 5);
    let summons = events(&run, CombatLogKind::Summon, 434400);
    assert_eq!(summons.len(), 1, "consumed Pit Lord Art must summon one real Pit Lord");
    let pit_lord = Entity::from_bits(summons[0].target.unwrap());
    assert!(
        run.app
            .world()
            .get::<crate::spell_cast::summon::SummonedCreature>(pit_lord)
            .is_some_and(|summon| summon.owner == run.actor && summon.spell_id == 434400),
        "the Pit Lord must be a live native summon of the follower"
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 432795), "Hand must consume the Art");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, RUINATION_BUFF),
        "Pit Lord summon must grant native Ruination"
    );
}

#[test]
#[ignore = "requires private266 inputs; Mother of Chaos grants Infernal Bolt"]
fn demonology_art_hand_summons_mother_and_grants_infernal_bolt() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    grant_art(&mut run, 432794);
    cast(&mut run, 105174);
    assert_eq!(events(&run, CombatLogKind::Summon, 428565).len(), 1, "Mother Art must summon one Mother");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF),
        "Mother summon must grant native Infernal Bolt"
    );
    let before = crate::spell_cast::powers::power_value(run.app.world(), run.actor, shared::components::PowerType::SoulShards).unwrap().0;
    let bolt = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 686);
    assert_eq!(bolt, 434506, "Infernal Bolt overrides Shadow Bolt");
    cast(&mut run, bolt);
    update(&mut run, 120);
    assert_eq!(events(&run, CombatLogKind::Damage, 434506).len(), 1, "Infernal Bolt must deal native damage");
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, INFERNAL_BOLT_BUFF), "the cast consumes the buff");
    let after = crate::spell_cast::powers::power_value(run.app.world(), run.actor, shared::components::PowerType::SoulShards).unwrap().0;
    assert!(after > before, "Infernal Bolt must generate native Soul Shards: {before} -> {after}");
}

#[test]
#[ignore = "requires private266 inputs; Ruination impact and its Hand meteor"]
fn demonology_ruination_impacts_then_summons_three_hand_imps() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_funded();
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, RUINATION_BUFF, Some(run.actor), true)
        .expect("native Ruination buff must apply");
    let ruination = crate::auras::overrides::resolve_spell(run.app.world(), run.actor, 105174);
    assert_eq!(ruination, 434635, "Ruination overrides Hand of Gul'dan");
    cast(&mut run, ruination);
    update(&mut run, 120);
    assert_eq!(events(&run, CombatLogKind::Damage, 434636).len(), 1, "Ruination impact must deal native damage");
    assert_eq!(events(&run, CombatLogKind::Damage, 86040).len(), 1, "Ruination must drop one Hand meteor");
    assert_eq!(
        events(&run, CombatLogKind::Summon, 104317).len(),
        3,
        "Ruination meteor must summon three actual Wild Imps"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, RUINATION_BUFF),
        "Ruination consumes its native buff"
    );
}
