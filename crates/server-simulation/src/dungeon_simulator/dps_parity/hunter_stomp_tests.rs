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
}
