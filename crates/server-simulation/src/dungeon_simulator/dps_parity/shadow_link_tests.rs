//! SimC76f8118 sc_priest_shadow.cpp1521–1619/2655–2669: own VT, other targets.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const LINK: u32 = 199486;
const TALENT: u32 = 199484;
const VT: u32 = 34914;
const MIND_BLAST: u32 = 8092;

fn cast_at(run: &mut Run, spell: u32, target: Entity) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, target, None),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_link_mind_blast_copies_only_to_other_owned_vampiric_touch() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage_fixture_opponents(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, TALENT));
    let primary = run.targets[0];
    let owned = run.targets[1];
    let foreign = run.targets[2];
    cast_at(&mut run, VT, owned);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: primary,
            target: foreign,
            spell_id: VT,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(foreign)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == VT && aura.caster == primary)
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast_at(&mut run, MIND_BLAST, primary);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.spell_id == Some(MIND_BLAST)
        && event.target == Some(primary.to_bits())
        && event.amount > 0));
    let copies = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(LINK)
                && event.source == Some(run.actor.to_bits())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        copies.len(),
        1,
        "one echo to owned VT, none to primary or foreign VT"
    );
    assert_eq!(copies[0].target, Some(owned.to_bits()));
    assert!(copies[0].amount > 0);
    assert!(!copies[0].crit, "Psychic Link cannot independently crit");
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_link_never_copies_ordinary_death_self_backlash_to_single_target() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    // A wounded caster avoids generating an overheal shield before the real backlash.
    {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap();
        health.current = health.max * 0.75;
    }
    cast_at(&mut run, VT, primary);
    {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(primary)
            .unwrap();
        health.current = health.max * 0.1; // Execute admission; the native dummy survives.
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast_at(&mut run, 32379, primary);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let death_damage = |target: Entity| {
        events
            .iter()
            .filter(|event| {
                let owned =
                    event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits());
                let hit = event.spell_id == Some(32379) && event.kind == CombatLogKind::Damage;
                owned && hit && event.amount > 0
            })
            .count()
    };
    assert_eq!(
        death_damage(primary),
        1,
        "ordinary hostile Death hit landed"
    );
    assert_eq!(
        death_damage(actor),
        1,
        "surviving dummy caused real self-backlash"
    );
    let copies = events
        .iter()
        .filter(|event| {
            let owned_link = event.source == Some(actor.to_bits()) && event.spell_id == Some(LINK);
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            owned_link && damage
        })
        .count();
    assert_eq!(
        copies, 0,
        "caster backlash is not a hostile Psychic Link producer"
    );
}
