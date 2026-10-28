use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn cast_builder(run: &mut Run) {
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(193315, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn unseen_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(441144)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_thousand_cuts_white_hits_reset_earned_unseen_cooldown() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 441346));
    cast_builder(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 459485));
    let first = unseen_hits(&run);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::combat::AutoAttack::new(run.targets[0]));
    for frame in 0..3000 {
        if frame % 20 == 0 {
            // Keep the real cooldown aura alive so natural expiration cannot pass this test.
            crate::spell_cast::cast_triggered(
                run.app.world_mut(),
                run.actor,
                459485,
                Some(run.actor),
                1,
            );
        }
        run.app.update();
        if !crate::auras::has_aura(run.app.world(), run.actor, 459485) {
            let white_hits = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.spell_id.is_none()
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count();
            assert!(
                white_hits > 0,
                "actual melee swings must earn the native RPPM proc"
            );
            cast_builder(&mut run);
            assert!(
                unseen_hits(&run) > first,
                "earned Thousand Cuts reset permits the next real Unseen Blade"
            );
            return;
        }
    }
    panic!(
        "selected Thousand Cuts never reset the continuously refreshed cooldown after real melee swings"
    );
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_sinister_earns_unseen_blade_and_owned_hero_auras() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 441146));
    cast_builder(&mut run);
    // SimC76f8118 sc_rogue.cpp5989/6689–6712/8821–8850: actual
    // Sinister execution earns a native Unseen hit and its owned hero state.
    assert!(
        unseen_hits(&run) > 0,
        "actual native Sinister Strike must launch Unseen Blade"
    );
    for spell in [459485, 441786, 441326] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, spell),
            "Unseen must earn native{spell}"
        );
    }
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap();
    assert!(
        auras
            .iter()
            .any(|aura| aura.spell_id == 441224 && aura.caster == run.actor),
        "actual Unseen impact must apply its own Fazed to the actual enemy"
    );
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_disorienting_uses_come_from_spree_not_passive_talent_presence() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 441274));
    cast_builder(&mut run);
    let first = unseen_hits(&run);
    assert!(first > 0);
    cast_builder(&mut run);
    assert_eq!(
        unseen_hits(&run),
        first,
        "selected passive Disorienting Strikes is not an earned runtime use"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 459485));
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 4.0;
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(51690, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 459485),
        "quiet cooldown must still be active for this boundary"
    );
    cast_builder(&mut run);
    // sc_rogue.cpp4812–4821/8829–8850: real Spree earns uses which
    // bypass the quiet cooldown, but never the native Unseen ICD.
    assert!(
        unseen_hits(&run) > first,
        "earned Spree use must permit the next real Unseen during its quiet cooldown"
    );
}
