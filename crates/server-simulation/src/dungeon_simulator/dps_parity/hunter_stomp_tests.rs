use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_barbed_stomps_the_pack_and_earns_mid2_cobra_fang() {
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 3).expect("native literal BM fixture");
    engage(&mut run);
    let pets: Vec<_> = run
        .app
        .world_mut()
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(run.app.world())
        .filter(|(_, pet)| pet.owner == run.actor)
        .map(|(unit, _)| unit)
        .collect();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        217200,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Barbed Shot");
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for pet in pets {
        for target in &run.targets {
            let attempts = events
                .iter()
                .filter(|event| {
                    event.source == Some(pet.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(201754)
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .count();
            assert_eq!(
                attempts, 2,
                "native Stomp and the selected2pc echo must reach each pack member: pet={pet}, target={target}"
            );
        }
    }
    let fang = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1299389 && aura.caster == run.actor)
        })
        .expect("native Stomp impacts must produce the selected4pc Cobra Fang aura");
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1299389)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura;
    assert_eq!(
        u32::from(fang.stacks),
        cap,
        "six or more native pack impacts must fill the actual aura's stack cap"
    );
    let stacks = f64::from(fang.stacks);
    let bonus = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1299389)
        .unwrap()
        .effects[1]
        .base_points;
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        193455,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Cobra Shot consumes earned Fang");
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1299389),
        "Cobra Shot must consume earned Fang on successful launch, before flight"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hit = events
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(193455)
                && event.kind == CombatLogKind::Damage
        })
        .unwrap();
    let normalized = f64::from(hit.amount) / if hit.crit { 2.0 } else { 1.0 };
    let expected = ordinary_cobra_damage(&run) * (1.0 + f64::from(bonus) * stacks / 100.0);
    // Native variation, post-cast procs and racial crit bonuses can change the
    // normalized hit slightly; an absent80% earned boost cannot fit this bound.
    assert!(
        (normalized - expected).abs() < expected * 0.15,
        "earned native Fang effectiveness must survive consumption and flight: actual={normalized}, expected={expected}"
    );
}

fn ordinary_cobra_damage(run: &Run) -> f64 {
    use server::simc_apl::Facts;
    let world = run.app.world();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("cobra_shot".into(), 193455)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let average =
        crate::simc_apl_native::NativeFacts::new(world, run.actor, &bindings, encounter, 3)
            .with_targets(Some(run.targets[0]), &run.targets)
            .value("action.cobra_shot.damage")
            .unwrap();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(193455)
        .unwrap();
    let crit = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct;
    let chance = crate::auras::spell_mods::modify(
        world,
        run.actor,
        spell,
        crate::auras::spell_mods::SpellModOp::CritChance,
        crit,
    ) / 100.0;
    average / (1.0 + f64::from(chance))
}
